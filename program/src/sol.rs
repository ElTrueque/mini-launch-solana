use pinocchio::{AccountView, Address, ProgramResult, error::ProgramError,
    instruction::{InstructionView, InstructionAccount}, cpi::{invoke_signed, Signer},
    sysvars::{Sysvar, rent::Rent}};
use crate::ids::*;
pub fn check(ok:bool,code:u32)->ProgramResult {if ok {Ok(())}else{Err(ProgramError::Custom(code))}}
pub fn read64(d:&[u8],i:usize)->Result<u64,ProgramError>{
    Ok(u64::from_le_bytes(d.get(i..i+8).ok_or(ProgramError::InvalidAccountData)?.try_into().map_err(|_|ProgramError::InvalidAccountData)?))
}
pub fn put64(d:&mut[u8],i:usize,v:u64){d[i..i+8].copy_from_slice(&v.to_le_bytes());}
pub fn address(d:&[u8],i:usize)->Address { Address::new_from_array(d[i..i+32].try_into().unwrap()) }
pub fn add(a:u64,b:u64)->Result<u64,ProgramError>{a.checked_add(b).ok_or(ProgramError::ArithmeticOverflow)}
pub fn sub(a:u64,b:u64)->Result<u64,ProgramError>{a.checked_sub(b).ok_or(ProgramError::InsufficientFunds)}
#[inline(never)]
pub fn invoke<const N:usize>(program:&Address,a:[&AccountView;N],flags:[(bool,bool);N],data:&[u8],signs:&[Signer])->ProgramResult{
    let metas:[InstructionAccount;N]=core::array::from_fn(|i|InstructionAccount::new(a[i].address(),flags[i].0,flags[i].1));
    invoke_signed(&InstructionView{program_id:program,accounts:&metas,data},&a,signs)
}
pub fn transfer_sol(from:&AccountView,to:&AccountView,n:u64,signs:&[Signer])->ProgramResult{
    let mut data=[0u8;12];data[..4].copy_from_slice(&2u32.to_le_bytes());data[4..].copy_from_slice(&n.to_le_bytes());
    invoke(&SYSTEM,[from,to],[(true,true),(true,false)],&data,signs)
}
pub fn allocate(account:&AccountView,len:usize,owner:&Address,funder:&AccountView,signs:&[Signer])->ProgramResult{
    check(account.owned_by(&SYSTEM)&&account.is_data_empty(),102)?;
    let mut data=[0u8;12];data[..4].copy_from_slice(&8u32.to_le_bytes());data[4..].copy_from_slice(&(len as u64).to_le_bytes());
    invoke(&SYSTEM,[account,funder],[(true,true),(true,false)],&data,signs)?;
    let mut data=[0u8;36];data[..4].copy_from_slice(&1u32.to_le_bytes());data[4..].copy_from_slice(owner.as_ref());
    invoke(&SYSTEM,[account,funder],[(true,true),(true,false)],&data,signs)
}
pub fn create_from_wallet(account:&AccountView,len:usize,owner:&Address,payer:&AccountView,signs:&[Signer])->ProgramResult{
    check(account.owned_by(&SYSTEM)&&account.is_data_empty(),102)?;
    if account.lamports()==0{
        let mut data=[0u8;52];
        data[4..12].copy_from_slice(&Rent::get()?.try_minimum_balance(len)?.to_le_bytes());
        data[12..20].copy_from_slice(&(len as u64).to_le_bytes());data[20..].copy_from_slice(owner.as_ref());
        return invoke(&SYSTEM,[payer,account],[(true,true),(true,true)],&data,signs);
    }
    let need=Rent::get()?.try_minimum_balance(len)?.saturating_sub(account.lamports());
    if need>0 {transfer_sol(payer,account,need,signs)?;}
    allocate(account,len,owner,payer,signs)
}
pub fn move_owned(from:&mut AccountView,to:&mut AccountView,n:u64)->ProgramResult{
    check(from.address()!=to.address(),103)?;
    let f=sub(from.lamports(),n)?;let t=add(to.lamports(),n)?;
    from.set_lamports(f);to.set_lamports(t);Ok(())
}
pub fn token_amount(a:&AccountView,mint:&Address,owner:&Address,strict:bool)->Result<u64,ProgramError>{
    check(a.owned_by(&TOKEN)&&a.data_len()==165,104)?;
    let d=a.try_borrow()?;
    check(&d[..32]==mint.as_ref()&&&d[32..64]==owner.as_ref()&&d[108]==1,105)?;
    if strict {check(d[72..76]==[0;4]&&d[129..133]==[0;4],106)?;}
    read64(&d,64)
}
pub fn initialize_token(vault:&AccountView,mint:&AccountView,owner:&Address)->ProgramResult{
    let mut data=[0u8;33];data[0]=18;data[1..].copy_from_slice(owner.as_ref());
    invoke(&TOKEN,[vault,mint],[(true,false),(false,false)],&data,&[])
}
pub fn transfer_token(from:&AccountView,to:&AccountView,authority:&AccountView,n:u64,signs:&[Signer])->ProgramResult{
    let mut data=[0u8;9];data[0]=3;data[1..].copy_from_slice(&n.to_le_bytes());
    invoke(&TOKEN,[from,to,authority],[(true,false),(true,false),(false,true)],&data,signs)
}
pub fn ata(owner:&Address,mint:&Address)->Address{
    Address::find_program_address(&[owner.as_ref(),TOKEN.as_ref(),mint.as_ref()],&ASSOCIATED).0
}
