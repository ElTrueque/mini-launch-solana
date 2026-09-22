//! Atomic pool creation, LP lock and custody. No external signature is needed except gas payer.
use pinocchio::{AccountView,Address,ProgramResult,error::ProgramError,cpi::{Signer,Seed},sysvars::{Sysvar,rent::Rent,clock::Clock}};
use crate::{ids::*,sol::*,load,solvent,settle,ALLOCATION,fees};

fn create_ata(payer:&AccountView,account:&AccountView,owner:&AccountView,mint:&AccountView,system:&AccountView,token:&AccountView,signs:&[Signer])->ProgramResult{
    check(account.address()==&ata(owner.address(),mint.address()),180)?;
    invoke(&ASSOCIATED,[payer,account,owner,mint,system,token],[(true,true),(true,false),(false,false),(false,false),(false,false),(false,false)],&[1],signs)
}
// 0..23: fee collection binding (see fees.rs); 24 launch; 25 authority; 26 launch token vault;
// 27 launch WSOL; 28 authority LP ATA; 29 pool config; 30 pool fee account; 31 observation;
// 32 ATA program; 33 rent; 34 metadata program; 35 fee NFT metadata; 36 treasury token ATA.
#[inline(never)]
pub fn graduate(program:&Address,a:&mut[AccountView],data:&[u8])->ProgramResult{
    check(a.len()==37&&(data==[5]||data==[7]),181)?;
    let mut d=load(program,&a[24])?;solvent(&a[24],&d)?;
    let target=read64(&d,296)?;
    check(d[232]!=2&&read64(&d,248)?==target,182)?;
    for (i,key) in [(0,LOCK_AUTH),(4,CPMM),(15,TOKEN),(16,TOKEN2022),(17,MEMO),(18,LOCK),(20,TREASURY),(23,SYSTEM),(29,RAY_CONFIG),(30,RAY_CREATE_FEE),(32,ASSOCIATED),(33,RENT),(34,METADATA)]{
        check(a[i].address()==&key,183)?;
    }
    for (i,offset) in [(1,168),(6,200),(19,8),(22,136),(25,104),(26,72)]{check(a[i].address()==&address(&d,offset),184)?;}
    check(a[25].owned_by(&SYSTEM)&&a[25].is_data_empty(),185)?;
    let state=*a[24].address();let mint=address(&d,40);let auth=*a[25].address();
    let ni=if a[12].address()==&WSOL{0}else{1};let ti=1-ni;
    check(a[12+ni].address()==&WSOL&&a[12+ti].address()==&mint,186)?;
    check(token_amount(&a[26],&mint,&auth,true)?>=ALLOCATION,187)?;
    let (quote,qb)=Address::find_program_address(&[b"quote",state.as_ref()],program);
    check(a[27].address()==&quote&&a[28].address()==&ata(&auth,a[7].address()),188)?;
    check(a[21].address()==&ata(&auth,a[22].address())&&a[2].address()==&ata(a[1].address(),a[22].address()),189)?;
    check(a[36].address()==&ata(&TREASURY,&mint),190)?;
    let ab=[d[236]];let pb=[d[237]];let nb=[d[238]];let qbb=[qb];
    let aus=[Seed::from(b"authority"),Seed::from(state.as_ref()),Seed::from(&ab)];
    let ps=[Seed::from(b"pool"),Seed::from(state.as_ref()),Seed::from(&pb)];
    let ns=[Seed::from(b"fee-nft"),Seed::from(state.as_ref()),Seed::from(&nb)];
    let qs=[Seed::from(b"quote"),Seed::from(state.as_ref()),Seed::from(&qbb)];
    let signs=[Signer::from(&aus)];
    if data==[7]{
        if d[241]==1{return Ok(())}
        return prepare(program,a,&mut d,ni,&[Signer::from(&aus),Signer::from(&qs)]);
    }
    check(d[241]==1,194)?;
    // Only the inscription provision funds CPI account creation. Principal stays separate.
    let budget=read64(&d,288)?;
    let mut launch=a[24];let mut authority=a[25];let mut quote_view=a[27];
    move_owned(&mut launch,&mut authority,budget)?;
    transfer_sol(&a[25],&a[24],0,&signs)?; // synchronize both balances before subsequent CPI
    token_amount(&a[27],&WSOL,&auth,true)?;
    move_owned(&mut launch,&mut quote_view,target)?;
    // Synchronize direct funding through System; SPL SyncNative accepts only its vault.
    let mut sync=[0u8;12];sync[..4].copy_from_slice(&2u32.to_le_bytes());
    invoke(&SYSTEM,[&a[25],&a[24],&a[27]],[(true,true),(true,false),(true,false)],&sync,&signs)?;
    invoke(&TOKEN,[&a[27]],[(true,false)],&[17],&[])?;
    create_pool(a,ni,target,&[Signer::from(&aus),Signer::from(&ps)])?;
    let lp=token_amount(&a[28],a[7].address(),&auth,true)?;check(lp>0,192)?;
    lock_pool(a,lp,&[Signer::from(&aus),Signer::from(&ns)])?;
    check(token_amount(&a[28],a[7].address(),&auth,true)?==0,193)?;
    let mut bound:[AccountView;24]=core::array::from_fn(|i|a[i]);
    fees::initialize_from_launch(program,&mut bound,&a[25],&signs)?;
    // Reclaim temporary account rent and unused budget, but retain pool and fee custody.
    for i in [27,28]{invoke(&TOKEN,[&a[i],&a[25],&a[25]],[(true,false),(true,false),(false,true)],&[9],&signs)?;}
    let leftover=token_amount(&a[26],&mint,&auth,true)?;
    if leftover>0{
        token_amount(&a[36],&mint,&TREASURY,false)?;
        transfer_token(&a[26],&a[36],&a[25],leftover,&signs)?;
    }
    transfer_sol(&a[25],&a[24],a[25].lamports(),&signs)?;
    put64(&mut d,248,0);put64(&mut d,256,0);put64(&mut d,288,0);
    put64(&mut d,280,0); // terminal odd lamport becomes part of treasury surplus
    let liabilities=add(Rent::get()?.try_minimum_balance(crate::LEN)?,add(read64(&d,264)?,read64(&d,272)?)?)?;
    let surplus=sub(a[24].lamports(),liabilities)?;
    let treasury_total=add(read64(&d,272)?,surplus)?;put64(&mut d,272,treasury_total);
    let mut creator=a[19];let mut treasury=a[20];
    settle(&mut launch,&mut d,&mut creator,&mut treasury)?;
    d[232]=2;put64(&mut d,336,u64::try_from(Clock::get()?.unix_timestamp).map_err(|_|ProgramError::InvalidAccountData)?);
    solvent(&a[24],&d)?;a[24].try_borrow_mut()?.copy_from_slice(&d);Ok(())
}
// Preparing rent-funded accounts cannot move liquidity principal or issue the Fee Key.
// It is repeatable, permissionless and uses only the inscription reserve.
#[inline(never)]
fn prepare(program:&Address,a:&mut[AccountView],d:&mut[u8;crate::LEN],ni:usize,signs:&[Signer])->ProgramResult{
    let ti=1-ni;let budget=read64(d,288)?;let mut launch=a[24];let mut authority=a[25];
    move_owned(&mut launch,&mut authority,budget)?;transfer_sol(&a[25],&a[24],0,&signs[..1])?;
    create_from_wallet(&a[27],165,&TOKEN,&a[25],signs)?;
    initialize_token(&a[27],&a[12+ni],a[25].address())?;
    let fees=*a[1].address();let nft=*a[22].address();let bb=[d[239]];
    let fs=[Seed::from(b"fees"),Seed::from(nft.as_ref()),Seed::from(&bb)];
    create_from_wallet(&a[1],800,program,&a[25],&[signs[0].clone(),Signer::from(&fs)])?;
    let (native,nb)=Address::find_program_address(&[b"wsol",fees.as_ref()],program);let nb=[nb];
    check(a[8+ni].address()==&native,191)?;
    let ns=[Seed::from(b"wsol"),Seed::from(fees.as_ref()),Seed::from(&nb)];
    create_from_wallet(&a[8+ni],165,&TOKEN,&a[25],&[signs[0].clone(),Signer::from(&ns)])?;
    initialize_token(&a[8+ni],&a[12+ni],&fees)?;
    create_ata(&a[25],&a[8+ti],&a[1],&a[12+ti],&a[23],&a[15],&signs[..1])?;
    create_ata(&a[25],&a[36],&a[20],&a[12+ti],&a[23],&a[15],&signs[..1])?;
    transfer_sol(&a[25],&a[24],a[25].lamports(),&signs[..1])?;
    let mut tracked=Rent::get()?.try_minimum_balance(crate::LEN)?;
    for i in [248,264,272,280]{tracked=add(tracked,read64(d,i)?)?;}
    let left=sub(a[24].lamports(),tracked)?;put64(d,288,left);d[241]=1;
    solvent(&a[24],d)?;a[24].try_borrow_mut()?.copy_from_slice(d);Ok(())
}
#[inline(never)]
fn create_pool(a:&[AccountView],ni:usize,target:u64,signs:&[Signer])->ProgramResult{
    let source_a=if ni==0{27}else{26};let source_b=if ni==1{27}else{26};
    let idx=[25,29,5,6,12,13,7,source_a,source_b,28,10,11,30,31,15,15,15,32,23,33];
    let views:[&AccountView;20]=core::array::from_fn(|i|&a[idx[i]]);
    let rights=core::array::from_fn(|i|(matches!(i,0|3|6|7|8|9|10|11|12|13),i==0||i==3));
    let mut data=[0u8;32];data[..8].copy_from_slice(&[175,175,109,31,13,152,155,237]);
    put64(&mut data,8,if ni==0{target}else{ALLOCATION});put64(&mut data,16,if ni==1{target}else{ALLOCATION});
    invoke(&CPMM,views,rights,&data,signs)
}
#[inline(never)]
fn lock_pool(a:&[AccountView],lp:u64,signs:&[Signer])->ProgramResult{
    let idx=[0,25,25,1,22,2,6,3,7,28,14,10,11,35,33,23,15,32,34];
    let views:[&AccountView;19]=core::array::from_fn(|i|&a[idx[i]]);
    let rights=core::array::from_fn(|i|(matches!(i,1|4|5|7|9|10|11|12|13),matches!(i,1|2|4)));
    // The non-transferable custodial Fee Key does not need display metadata.
    // Omitting optional metadata preserves the network's CPI trace budget even for prefunded PDAs.
    let mut data=[0u8;17];data[..8].copy_from_slice(&[216,157,29,78,38,51,31,26]);put64(&mut data,8,lp);
    invoke(&LOCK,views,rights,&data,signs)
}
// Permissionless native/token surplus and deferred-credit settlement, fixed destinations.
// state, creator, treasury, mint, token vault, authority, treasury token ATA, SPL.
pub fn sweep(program:&Address,a:&mut[AccountView],data:&[u8])->ProgramResult{
    check(a.len()==8&&data==[6],195)?;let mut d=load(program,&a[0])?;
    check(d[232]==2&&a[1].address()==&address(&d,8)&&a[2].address()==&TREASURY,196)?;
    for (i,offset) in [(3,40),(4,72),(5,104)]{check(a[i].address()==&address(&d,offset),197)?;}
    check(a[7].address()==&TOKEN&&a[6].address()==&ata(&TREASURY,a[3].address()),198)?;
    let state=*a[0].address();let ab=[d[236]];let aus=[Seed::from(b"authority"),Seed::from(state.as_ref()),Seed::from(&ab)];
    let amount=token_amount(&a[4],a[3].address(),a[5].address(),true)?;
    if amount>0{transfer_token(&a[4],&a[6],&a[5],amount,&[Signer::from(&aus)])?;}
    let liabilities=add(Rent::get()?.try_minimum_balance(crate::LEN)?,add(read64(&d,264)?,read64(&d,272)?)?)?;
    let surplus=sub(a[0].lamports(),liabilities)?;
    let t=add(read64(&d,272)?,surplus)?;put64(&mut d,272,t);
    let mut launch=a[0];let mut creator=a[1];let mut treasury=a[2];settle(&mut launch,&mut d,&mut creator,&mut treasury)?;
    solvent(&a[0],&d)?;a[0].try_borrow_mut()?.copy_from_slice(&d);Ok(())
}
