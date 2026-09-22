//! Fee custody integrated with the launch program; initialization is internal to graduation.
//! No instruction releases the NFT, project-token receipts, or LP principal.
use pinocchio::{AccountView, Address, ProgramResult, error::ProgramError,
    instruction::{InstructionView, InstructionAccount},
    cpi::{invoke_signed, Signer, Seed}, sysvars::{Sysvar, rent::Rent}};
use crate::ids::*;
const LEN: usize = 800;
const MAGIC: &[u8;8] = b"ETFEES01";
// 24 bound addresses follow MAGIC. Bytes 776/777: state/WSOL bumps; 778: odd lamport.
// u64 credits at 784/792 preserve payments below recipient rent-exemption minimum.
// Accounts 0..17 are Raydium collect_cp_fee's accounts (1 = our state PDA).
// 18 lock program, 19 creator, 20 treasury, 21 source NFT (init only), 22 NFT mint, 23 system.
fn require(ok: bool, code: u32) -> ProgramResult {
    if ok { Ok(()) } else { Err(ProgramError::Custom(code)) }
}
fn u64_at(d:&[u8], i:usize)->Result<u64,ProgramError>{
    Ok(u64::from_le_bytes(d.get(i..i+8).ok_or(ProgramError::InvalidAccountData)?.try_into().map_err(|_|ProgramError::InvalidAccountData)?))
}
fn token(a:&AccountView,mint:&Address,owner:&Address)->Result<u64,ProgramError>{
    require(a.owned_by(&TOKEN) && a.data_len()==165, 10)?;
    let d=a.try_borrow()?;
    require(&d[0..32]==mint.as_ref() && &d[32..64]==owner.as_ref() && d[108]==1,11)?;
    require(d[72..76]==[0;4] && d[129..133]==[0;4],12)?;
    u64_at(&d,64)
}
fn ata(owner:&Address,mint:&Address)->Address{
    Address::find_program_address(&[owner.as_ref(),TOKEN.as_ref(),mint.as_ref()],&ASSOCIATED).0
}
fn ix<const N:usize>(program:&Address, accs:[&AccountView;N], rights:[(bool,bool);N], data:&[u8], signs:&[Signer])->ProgramResult{
    let metas: [InstructionAccount;N] = core::array::from_fn(|i|InstructionAccount::new(accs[i].address(),rights[i].0,rights[i].1));
    invoke_signed(&InstructionView{program_id:program,accounts:&metas,data},&accs,signs)
}
fn allocate(a:&AccountView,len:usize,owner:&Address,balance_source:&AccountView,signs:&[Signer])->ProgramResult{
    require(a.owned_by(&SYSTEM) && a.is_data_empty(),13)?;
    let mut d=[0u8;12];d[..4].copy_from_slice(&8u32.to_le_bytes());d[4..].copy_from_slice(&(len as u64).to_le_bytes());
    // Include the funding account so the runtime synchronizes both sides of any
    // preceding direct lamport movement before entering the child instruction.
    ix(&SYSTEM,[a,balance_source],[(true,true),(true,false)],&d,signs)?;
    let mut d=[0u8;36];d[..4].copy_from_slice(&1u32.to_le_bytes());d[4..].copy_from_slice(owner.as_ref());
    ix(&SYSTEM,[a,balance_source],[(true,true),(true,false)],&d,signs)
}
fn init_native(vault:&AccountView,mint:&AccountView,authority:&AccountView,signs:&[Signer])->ProgramResult{
    allocate(vault,165,&TOKEN,authority,signs)?;
    let mut d=[0u8;33];d[0]=18;d[1..].copy_from_slice(authority.address().as_ref());
    ix(&TOKEN,[vault,mint],[(true,false),(false,false)],&d,&[])
}
fn move_owned(from:&mut AccountView,to:&mut AccountView,n:u64)->ProgramResult{
    require(from.address()!=to.address(),14)?;
    let next_from=from.lamports().checked_sub(n).ok_or(ProgramError::InsufficientFunds)?;
    let next_to=to.lamports().checked_add(n).ok_or(ProgramError::ArithmeticOverflow)?;
    from.set_lamports(next_from);to.set_lamports(next_to);Ok(())
}
pub fn process_instruction(program:&Address,a:&mut[AccountView],data:&[u8])->ProgramResult{
    require(a.len()==24 && data.len()==1,1)?;
    require(a[18].address()==&LOCK && a[18].executable() && a[4].address()==&CPMM && a[4].executable(),2)?;
    require(a[15].address()==&TOKEN && a[16].address()==&TOKEN2022 && a[17].address()==&MEMO && a[23].address()==&SYSTEM,3)?;
    require(a[20].address()==&TREASURY,4)?;
    require(a[0].address()==&LOCK_AUTH,5)?;
    match data[0]{65=>collect(program,a),_=>Err(ProgramError::InvalidInstructionData)}
}
#[inline(never)]
pub fn initialize_from_launch(program:&Address,a:&mut[AccountView],_payer:&AccountView,_payer_signs:&[Signer])->ProgramResult{
    require(a[1].owned_by(program) && a[1].data_len()==LEN,7)?;
    require(a[1].try_borrow()?.iter().all(|n|*n==0),7)?;
    require(a[6].owned_by(&CPMM) && a[3].owned_by(&LOCK),8)?;
    let (state,bump)=Address::find_program_address(&[b"fees",a[22].address().as_ref()],program);
    require(a[1].address()==&state,9)?;
    let native_index=if a[12].address()==&WSOL {0}else{require(a[13].address()==&WSOL,15)?;1};
    let token_index=1-native_index;
    let project=*a[12+token_index].address();
    require(project!=WSOL && a[12+token_index].owned_by(&TOKEN),16)?;
    {
        let m=a[12+token_index].try_borrow()?;
        require(m.len()==82 && m[..4]==[0;4] && m[44]==9 && m[45]==1 && m[46..50]==[0;4],17)?;
        require(u64_at(&m,36)?==1_000_000_000_000_000,18)?;
    }
    require(a[22].owned_by(&TOKEN),19)?;
    {let n=a[22].try_borrow()?;require(n.len()==82 && n[44]==0 && u64_at(&n,36)?==1,20)?;}
    // Raydium mints the Fee Key directly into our custody during the same instruction.
    require(a[2].address()==&ata(&state,a[22].address()) && token(&a[2],a[22].address(),&state)?==1,22)?;
    require(a[8+token_index].address()==&ata(&state,&project),23)?;
    token(&a[8+token_index],&project,&state)?;
    let lock=Address::find_program_address(&[b"locked_liquidity",a[22].address().as_ref()],&LOCK).0;
    require(a[3].address()==&lock,24)?;
    let (native,nbump)=Address::find_program_address(&[b"wsol",state.as_ref()],program);
    require(a[8+native_index].address()==&native,25)?;
    {let p=a[6].try_borrow()?;
     require(p.len()>=328,26)?;
     for (offset,index) in [(72,10),(104,11),(136,7),(168,12),(200,13)]{
        require(&p[offset..offset+32]==a[index].address().as_ref(),27)?;
     }
    }
    token(&a[8+native_index],&WSOL,&state)?;
    {
        let mut bound=[0u8;LEN];bound[..8].copy_from_slice(MAGIC);
        for i in 0..24{bound[8+32*i..40+32*i].copy_from_slice(a[i].address().as_ref());}
        bound[776]=bump;bound[777]=nbump;
        a[1].try_borrow_mut()?.copy_from_slice(&bound);
    }
    Ok(())
}
#[inline(never)]
fn collect(program:&Address,a:&mut[AccountView])->ProgramResult{
    require(a[1].owned_by(program) && a[1].data_len()==LEN,30)?;
    let (bump,nbump,carry,old_creator,old_treasury)={let d=a[1].try_borrow()?;
        require(&d[..8]==MAGIC && d[778]<=1,31)?;
        for i in 0..24{if i!=21 {require(&d[8+32*i..40+32*i]==a[i].address().as_ref(),32)?;}}
        (d[776],d[777],d[778] as u64,u64_at(&d,784)?,u64_at(&d,792)?)
    };
    let state=*a[1].address();let nft=*a[22].address();
    let bs=[bump];let ns=[nbump];
    let seeds=[Seed::from(b"fees"),Seed::from(nft.as_ref()),Seed::from(&bs)];
    let signs=[Signer::from(&seeds)];
    require(Address::create_program_address(&[b"fees",nft.as_ref(),&bs],program).map_err(|_|ProgramError::InvalidSeeds)?==state,33)?;
    let ni=if a[12].address()==&WSOL{0}else{1};
    require(token(&a[2],&nft,&state)?==1,34)?;
    token(&a[8+ni],&WSOL,&state)?;
    token(&a[9-ni],a[13-ni].address(),&state)?;
    let mut cp=[0u8;16];cp[..8].copy_from_slice(&[8,30,51,199,209,184,247,133]);cp[8..].copy_from_slice(&u64::MAX.to_le_bytes());
    let views:[&AccountView;18]=core::array::from_fn(|i|&a[i]);
    let rights:[(bool,bool);18]=core::array::from_fn(|i|(matches!(i,2|3|6|7|8|9|10|11|14),i==1));
    ix(&LOCK,views,rights,&cp,&signs)?;
    ix(&TOKEN,[&a[8+ni]],[(true,false)],&[17],&[])?;
    let fees=token(&a[8+ni],&WSOL,&state)?;
    let prior_liabilities=carry.checked_add(old_creator).and_then(|n|n.checked_add(old_treasury)).ok_or(ProgramError::ArithmeticOverflow)?;
    let reserved=a[1].lamports().checked_sub(prior_liabilities).ok_or(ProgramError::InsufficientFunds)?;
    let mut state_view=a[1];let mut native_view=a[8+ni];
    let rent=Rent::get()?;
    if fees>0 {
        ix(&TOKEN,[&a[8+ni],&a[1],&a[1]],[(true,false),(true,false),(false,true)],&[9],&signs)?;
        let native_seeds=[Seed::from(b"wsol"),Seed::from(state.as_ref()),Seed::from(&ns)];
        move_owned(&mut state_view,&mut native_view,rent.try_minimum_balance(165)?)?;
        init_native(&a[8+ni],&a[12+ni],&a[1],&[Signer::from(&native_seeds)])?;
    }
    let total=fees.checked_add(carry).ok_or(ProgramError::ArithmeticOverflow)?;
    let half=total/2;
    let mut creator=a[19];let mut treasury=a[20];
    let mut creator_credit=old_creator.checked_add(half).ok_or(ProgramError::ArithmeticOverflow)?;
    let mut treasury_credit=old_treasury.checked_add(half).ok_or(ProgramError::ArithmeticOverflow)?;
    if creator.address()==treasury.address(){
        let combined=creator_credit.checked_add(treasury_credit).ok_or(ProgramError::ArithmeticOverflow)?;
        if combined>0 && creator.lamports().checked_add(combined).ok_or(ProgramError::ArithmeticOverflow)? >= rent.try_minimum_balance(creator.data_len())? {
            move_owned(&mut state_view,&mut creator,combined)?;creator_credit=0;treasury_credit=0;
        }
    }else{
        if creator_credit>0 && creator.lamports().checked_add(creator_credit).ok_or(ProgramError::ArithmeticOverflow)? >= rent.try_minimum_balance(creator.data_len())? {
            move_owned(&mut state_view,&mut creator,creator_credit)?;creator_credit=0;
        }
        if treasury_credit>0 && treasury.lamports().checked_add(treasury_credit).ok_or(ProgramError::ArithmeticOverflow)? >= rent.try_minimum_balance(treasury.data_len())? {
            move_owned(&mut state_view,&mut treasury,treasury_credit)?;treasury_credit=0;
        }
    }
    let retained=reserved.checked_add(total%2).and_then(|n|n.checked_add(creator_credit)).and_then(|n|n.checked_add(treasury_credit)).ok_or(ProgramError::ArithmeticOverflow)?;
    require(a[1].lamports()>=retained,35)?;
    let mut d=a[1].try_borrow_mut()?;
    d[778]=(total%2) as u8;
    d[784..792].copy_from_slice(&creator_credit.to_le_bytes());
    d[792..800].copy_from_slice(&treasury_credit.to_le_bytes());
    Ok(())
}
