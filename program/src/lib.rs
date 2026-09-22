// SPDX-License-Identifier: MIT
/*
    ███████╗██╗
    ██╔════╝██║
    █████╗  ██║
    ██╔══╝  ██║
    ███████╗███████╗
    ╚══════╝╚══════╝

    ████████╗██████╗ ██╗   ██╗███████╗ ██████╗ ██╗   ██╗███████╗
    ╚══██╔══╝██╔══██╗██║   ██║██╔════╝██╔═══██╗██║   ██║██╔════╝
       ██║   ██████╔╝██║   ██║█████╗  ██║   ██║██║   ██║█████╗
       ██║   ██╔══██╗██║   ██║██╔══╝  ██║▄▄ ██║██║   ██║██╔══╝
       ██║   ██║  ██║╚██████╔╝███████╗╚██████╔╝╚██████╔╝███████╗
       ╚═╝   ╚═╝  ╚═╝ ╚═════╝ ╚══════╝ ╚══▀▀═╝  ╚═════╝ ╚══════╝

    ███╗   ███╗██╗███╗   ██╗██╗
    ████╗ ████║██║████╗  ██║██║
    ██╔████╔██║██║██╔██╗ ██║██║
    ██║╚██╔╝██║██║██║╚██╗██║██║
    ██║ ╚═╝ ██║██║██║ ╚████║██║
    ╚═╝     ╚═╝╚═╝╚═╝  ╚═══╝╚═╝

    ██╗      █████╗ ██╗   ██╗███╗   ██╗ ██████╗██╗  ██╗
    ██║     ██╔══██╗██║   ██║████╗  ██║██╔════╝██║  ██║
    ██║     ███████║██║   ██║██╔██╗ ██║██║     ███████║
    ██║     ██╔══██║██║   ██║██║╚██╗██║██║     ██╔══██║
    ███████╗██║  ██║╚██████╔╝██║ ╚████║╚██████╗██║  ██║
    ╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝ ╚═════╝╚═╝  ╚═╝

    Mini Launch by El Trueque  ·  https://el-trueque.com  ·  v1.0.0-solana
    Lanzamientos a precio fijo con liquidez bloqueada en Raydium CPMM y Burn & Earn.
*/

//! Mini Launch by El Trueque — Solana launch program.
#![no_std]
use pinocchio::{AccountView,Address,ProgramResult,error::ProgramError,cpi::{Seed,Signer},sysvars::{Sysvar,clock::Clock,rent::Rent}};
mod ids;mod sol;mod fees;mod graduation;use ids::*;use sol::*;
pinocchio::program_entrypoint!(process_instruction);
pinocchio::no_allocator!();pinocchio::nostd_panic_handler!();
pub const LEN:usize=384;
pub const SUPPLY:u64=1_000_000_000_000_000;
pub const ALLOCATION:u64=SUPPLY/2;
pub const INSCRIPTION:u64=400_000_000;
pub const SERVICE:u64=150_000_000;
pub const EXPENSES:u64=250_000_000;
pub const MAGIC:&[u8;8]=b"ETLAUN01";
// pubkeys: creator 8, mint 40, vault 72, authority 104, fee NFT 136, fee state 168, pool 200.
// status 232 (open/ready/graduated), mode 233, state/vault/authority/pool/NFT/fee bumps 234..239,
// reopened 240; u64: reserve 248, inventory 256, creator/treasury credits 264/272,
// remainder 280, graduation budget 288, target 296, cap 304, created 312, last trade 320,
// ready since 328, graduated at 336. Remaining bytes reserved, zeroed at creation.
fn process_instruction(program:&Address,a:&mut[AccountView],data:&[u8])->ProgramResult{
    match data.first(){
        Some(0)=>create(program,a,data),Some(1)=>trade(program,a,data,true),Some(2)=>trade(program,a,data,false),
        Some(3)=>claim(program,a,data),Some(4)=>reopen(program,a,data),
        Some(5)|Some(7)=>graduation::graduate(program,a,data),Some(6)=>graduation::sweep(program,a,data),
        Some(65)=>fees::process_instruction(program,a,data),
        _=>Err(ProgramError::InvalidInstructionData)
    }
}
pub fn load(program:&Address,a:&AccountView)->Result<[u8;LEN],ProgramError>{
    check(a.owned_by(program)&&a.data_len()==LEN,110)?;
    let data=a.try_borrow()?;check(&data[..8]==MAGIC,111)?;
    let d:[u8;LEN]=data[..].try_into().map_err(|_|ProgramError::InvalidAccountData)?;
    let mint=address(&d,40);let bump=[d[234]];
    check(Address::create_program_address(&[b"launch",mint.as_ref(),&bump],program).map_err(|_|ProgramError::InvalidSeeds)?==*a.address(),112)?;
    Ok(d)
}
pub fn solvent(a:&AccountView,d:&[u8;LEN])->ProgramResult{
    let mut need=Rent::get()?.try_minimum_balance(LEN)?;
    for offset in [248,264,272,280,288]{need=add(need,read64(d,offset)?)?;}
    check(a.lamports()>=need,113)
}
pub fn settle(state:&mut AccountView,d:&mut[u8;LEN],creator:&mut AccountView,treasury:&mut AccountView)->ProgramResult{
    check(creator.address()==&address(d,8)&&treasury.address()==&TREASURY,114)?;
    let c=read64(d,264)?;let t=read64(d,272)?;let rent=Rent::get()?;
    if creator.address()==treasury.address(){
        let total=add(c,t)?;
        if total>0&&add(creator.lamports(),total)? >= rent.try_minimum_balance(creator.data_len())?{
            move_owned(state,creator,total)?;put64(d,264,0);put64(d,272,0);
        }
    }else{
        if c>0&&add(creator.lamports(),c)? >= rent.try_minimum_balance(creator.data_len())?{move_owned(state,creator,c)?;put64(d,264,0);}
        if t>0&&add(treasury.lamports(),t)? >= rent.try_minimum_balance(treasury.data_len())?{move_owned(state,treasury,t)?;put64(d,272,0);}
    }Ok(())
}
fn accrue(d:&mut[u8;LEN],fee:u64)->ProgramResult{
    let total=add(fee,read64(d,280)?)?;let c=add(read64(d,264)?,total/2)?;let t=add(read64(d,272)?,total/2)?;
    put64(d,264,c);put64(d,272,t);put64(d,280,total%2);Ok(())
}
fn now()->Result<u64,ProgramError>{u64::try_from(Clock::get()?.unix_timestamp).map_err(|_|ProgramError::InvalidAccountData)}
// Creation accounts: creator, state, mint, token vault, authority, treasury, SPL, system,
// Metaplex, token metadata, rent. Data: 0, mode, name_len, symbol_len, uri_len u16, strings.
#[inline(never)]
fn create(program:&Address,a:&mut[AccountView],data:&[u8])->ProgramResult{
    check(a.len()==11&&data.len()>=6,120)?;check(a[0].is_signer()&&a[2].is_signer(),121)?;
    check(a[5].address()==&TREASURY&&a[6].address()==&TOKEN&&a[7].address()==&SYSTEM&&a[8].address()==&METADATA&&a[10].address()==&RENT,122)?;
    check(data[1]==1||data[1]==2,123)?;
    let nl=data[2] as usize;let sl=data[3] as usize;let ul=u16::from_le_bytes([data[4],data[5]]) as usize;
    check(nl>0&&nl<=32&&sl>0&&sl<=10&&ul<=200&&data.len()==6+nl+sl+ul,124)?;
    check(core::str::from_utf8(&data[6..6+nl]).is_ok()&&core::str::from_utf8(&data[6+nl..6+nl+sl]).is_ok()&&core::str::from_utf8(&data[6+nl+sl..]).is_ok(),125)?;
    let mint=*a[2].address();let (state,sb)=Address::find_program_address(&[b"launch",mint.as_ref()],program);
    let (vault,vb)=Address::find_program_address(&[b"tokens",state.as_ref()],program);
    let (auth,ab)=Address::find_program_address(&[b"authority",state.as_ref()],program);
    check(a[1].address()==&state&&a[3].address()==&vault&&a[4].address()==&auth,126)?;
    let metadata=Address::find_program_address(&[b"metadata",METADATA.as_ref(),mint.as_ref()],&METADATA).0;
    check(a[9].address()==&metadata,127)?;
    let sbuf=[sb];let vbuf=[vb];let abuf=[ab];
    let ss=[Seed::from(b"launch"),Seed::from(mint.as_ref()),Seed::from(&sbuf)];
    let vs=[Seed::from(b"tokens"),Seed::from(state.as_ref()),Seed::from(&vbuf)];
    let aus=[Seed::from(b"authority"),Seed::from(state.as_ref()),Seed::from(&abuf)];
    let auth_sign=[Signer::from(&aus)];
    create_from_wallet(&a[1],LEN,program,&a[0],&[Signer::from(&ss)])?;
    create_from_wallet(&a[2],82,&TOKEN,&a[0],&[])?;
    let mut init_mint=[0u8;35];init_mint[0]=20;init_mint[1]=9;init_mint[2..34].copy_from_slice(auth.as_ref());
    invoke(&TOKEN,[&a[2]],[(true,false)],&init_mint,&[])?;
    create_from_wallet(&a[3],165,&TOKEN,&a[0],&[Signer::from(&vs)])?;
    initialize_token(&a[3],&a[2],&auth)?;
    let mut mint_to=[0u8;9];mint_to[0]=7;mint_to[1..].copy_from_slice(&SUPPLY.to_le_bytes());
    invoke(&TOKEN,[&a[2],&a[3],&a[4]],[(true,false),(true,false),(false,true)],&mint_to,&auth_sign)?;
    // Immutable Metaplex metadata, no royalty and no collection/uses/creator delegation.
    let mut md=[0u8;280];let mut n=1;md[0]=33;
    let mut pos=6;
    for count in [nl,sl,ul]{md[n..n+4].copy_from_slice(&(count as u32).to_le_bytes());n+=4;md[n..n+count].copy_from_slice(&data[pos..pos+count]);pos+=count;n+=count;}
    n+=7; // u16 seller fee; three None options; false mutable; None collection details.
    invoke(&METADATA,[&a[9],&a[2],&a[4],&a[0],&a[4],&a[7],&a[10]],[(true,false),(false,false),(false,true),(true,true),(false,false),(false,false),(false,false)],&md[..n],&auth_sign)?;
    invoke(&TOKEN,[&a[2],&a[4]],[(true,false),(false,true)],&[6,0,0],&auth_sign)?;
    transfer_sol(&a[0],&a[5],SERVICE,&[])?;
    transfer_sol(&a[0],&a[1],EXPENSES,&[])?;
    let mut d=[0u8;LEN];d[..8].copy_from_slice(MAGIC);
    for (offset,key) in [(8,*a[0].address()),(40,mint),(72,vault),(104,auth)]{d[offset..offset+32].copy_from_slice(key.as_ref());}
    let (pool,pb)=Address::find_program_address(&[b"pool",state.as_ref()],program);
    let (nft,nb)=Address::find_program_address(&[b"fee-nft",state.as_ref()],program);
    let (fees,fb)=Address::find_program_address(&[b"fees",nft.as_ref()],program);
    for (offset,key) in [(136,nft),(168,fees),(200,pool)]{d[offset..offset+32].copy_from_slice(key.as_ref());}
    d[233]=data[1];d[234]=sb;d[235]=vb;d[236]=ab;d[237]=pb;d[238]=nb;d[239]=fb;
    let target=data[1] as u64*10_000_000_000;let timestamp=now()?;
    for (offset,value) in [(256,ALLOCATION),(288,EXPENSES),(296,target),(304,target/10),(312,timestamp),(320,timestamp)]{put64(&mut d,offset,value);}
    solvent(&a[1],&d)?;a[1].try_borrow_mut()?.copy_from_slice(&d);Ok(())
}
// Trade accounts: wallet, launch, mint, launch vault, wallet token account, purchase PDA,
// authority, SPL token, system. Data: opcode, amount u64, minimum output u64.
#[inline(never)]
fn trade(program:&Address,a:&mut[AccountView],data:&[u8],buy:bool)->ProgramResult{
    check(a.len()==9&&data.len()==17,130)?;check(a[0].is_signer(),131)?;
    check(a[7].address()==&TOKEN&&a[8].address()==&SYSTEM,132)?;
    let mut d=load(program,&a[1])?;solvent(&a[1],&d)?;check(d[232]==0,133)?;
    let mint=address(&d,40);let state=*a[1].address();let auth=address(&d,104);
    check(a[2].address()==&mint&&a[3].address()==&address(&d,72)&&a[6].address()==&auth,134)?;
    let vault_balance=token_amount(&a[3],&mint,&auth,true)?;
    let user_balance=token_amount(&a[4],&mint,a[0].address(),false)?;
    let amount=read64(data,1)?;let minimum=read64(data,9)?;check(amount>0,135)?;
    let target=read64(&d,296)?;let reserve=read64(&d,248)?;let inventory=read64(&d,256)?;
    let ab=[d[236]];let aus=[Seed::from(b"authority"),Seed::from(state.as_ref()),Seed::from(&ab)];
    if buy{
        let wallet=*a[0].address();let (receipt,rb)=Address::find_program_address(&[b"purchased",state.as_ref(),wallet.as_ref()],program);
        check(a[5].address()==&receipt,136)?;
        let mut r=[0u8;80];
        if a[5].owned_by(&SYSTEM)&&a[5].is_data_empty(){
            let bb=[rb];let rs=[Seed::from(b"purchased"),Seed::from(state.as_ref()),Seed::from(wallet.as_ref()),Seed::from(&bb)];
            create_from_wallet(&a[5],80,program,&a[0],&[Signer::from(&rs)])?;
            r[..8].copy_from_slice(b"ETBUY001");r[8..40].copy_from_slice(state.as_ref());r[40..72].copy_from_slice(wallet.as_ref());
        }else{
            check(a[5].owned_by(program)&&a[5].data_len()==80,137)?;r.copy_from_slice(&a[5].try_borrow()?);
            check(&r[..8]==b"ETBUY001"&&&r[8..40]==state.as_ref()&&&r[40..72]==wallet.as_ref(),138)?;
        }
        let spent=read64(&r,72)?;check(amount<=sub(read64(&d,304)?,spent)?,139)?;
        let remaining=sub(target,reserve)?;let max_gross=if remaining==0{0}else{add(remaining,(remaining-1)/49)?};
        check(amount<=max_gross,140)?;
        let fee=amount/50;let net=amount-fee;let out=((net as u128*ALLOCATION as u128)/target as u128) as u64;
        check(out>0&&out>=minimum&&out<=inventory&&vault_balance>=add(ALLOCATION,out)?,141)?;
        put64(&mut r,72,add(spent,amount)?);
        let new_reserve=add(reserve,net)?;put64(&mut d,248,new_reserve);put64(&mut d,256,inventory-out);accrue(&mut d,fee)?;
        if new_reserve==target&&d[240]==0{d[232]=1;put64(&mut d,328,now()?);}
        transfer_sol(&a[0],&a[1],amount,&[])?;
        transfer_token(&a[3],&a[4],&a[6],out,&[Signer::from(&aus)])?;
        a[5].try_borrow_mut()?.copy_from_slice(&r);
    }else{
        let gross=((amount as u128*target as u128)/ALLOCATION as u128) as u64;let fee=gross/50;let net=gross-fee;
        check(gross>0&&amount<=ALLOCATION-inventory&&amount<=user_balance&&gross<=reserve&&net>=minimum,142)?;
        put64(&mut d,248,reserve-gross);put64(&mut d,256,add(inventory,amount)?);accrue(&mut d,fee)?;
        transfer_token(&a[4],&a[3],&a[0],amount,&[])?;
        let mut state_view=a[1];let mut user=a[0];move_owned(&mut state_view,&mut user,net)?;
    }
    put64(&mut d,320,now()?);solvent(&a[1],&d)?;a[1].try_borrow_mut()?.copy_from_slice(&d);Ok(())
}
// Beneficiary signature may select its own recipient, matching the existing EVM contract.
fn claim(program:&Address,a:&mut[AccountView],data:&[u8])->ProgramResult{
    check(a.len()==3&&data==[3]&&a[0].is_signer(),150)?;
    let mut d=load(program,&a[1])?;solvent(&a[1],&d)?;let mut amount=0;
    if a[0].address()==&address(&d,8){amount=add(amount,read64(&d,264)?)?;put64(&mut d,264,0);}
    if a[0].address()==&TREASURY{amount=add(amount,read64(&d,272)?)?;put64(&mut d,272,0);}
    check(amount>0&&a[2].address()!=a[1].address(),151)?;
    let mut state=a[1];let mut recipient=a[2];move_owned(&mut state,&mut recipient,amount)?;
    solvent(&a[1],&d)?;a[1].try_borrow_mut()?.copy_from_slice(&d);Ok(())
}
fn reopen(program:&Address,a:&mut[AccountView],data:&[u8])->ProgramResult{
    check(a.len()==1&&data==[4],160)?;let mut d=load(program,&a[0])?;
    check(d[232]==1,161)?;check(now()?>=add(read64(&d,328)?,30*24*60*60)?,162)?;
    d[232]=0;d[240]=1;put64(&mut d,328,0);a[0].try_borrow_mut()?.copy_from_slice(&d);Ok(())
}
