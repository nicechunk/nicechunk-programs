//! Fixed, fuel-free starter recipe. Inputs and output statistics are never supplied by clients.
use solana_program::{account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, pubkey::Pubkey, system_program, sysvar::Sysvar};
use crate::{state::*, errors::{NicechunkBackpackError as E, require_key_eq}, cluster_config::NICECHUNK_PLAYER_PROGRAM_ID};
pub const AXE_CODE: &[u8] = include_bytes!("starter_axe_code.bin");
pub const TWIG_BLOCK_ID: u16 = 54;
pub const AXE_DURABILITY: u32 = 99;

pub fn craft(program_id: &Pubkey, accounts: &[AccountInfo], payload: &[u8]) -> ProgramResult {
    if accounts.len()!=7 || payload.len()<11 { return Err(E::InvalidInstruction.into()); }
    let item_id=u64::from_le_bytes(payload[..8].try_into().map_err(|_|E::InvalidInstruction)?);
    let count=payload[8] as usize;
    if count<2 || count>3 || payload.len()!=9+count || item_id==0 {return Err(E::InvalidInstruction.into());}
    let owner=&accounts[0];let profile=&accounts[1];let backpack=&accounts[2];let item=&accounts[3];
    let player=&accounts[4];let physics=&accounts[5];let system=&accounts[6];
    if !owner.is_signer || !owner.is_writable {return Err(E::InvalidPayer.into());}
    if !backpack.is_writable || !item.is_writable {return Err(E::InvalidWritableAccount.into());}
    require_key_eq(player.key,&NICECHUNK_PLAYER_PROGRAM_ID,E::InvalidPlayerProgram)?;
    require_key_eq(profile.owner,player.key,E::InvalidPlayerProgram)?;
    require_key_eq(backpack.owner,program_id,E::InvalidBackpackOwner)?;
    require_key_eq(system.key,&system_program::ID,E::InvalidSystemProgram)?;
    let global={let data=profile.try_borrow_data()?;PlayerProfileView::validate_owner(&data,owner.key)?;PlayerProfileView::owner_and_global_config(&data)?.1};
    super::validate_existing_backpack_pda(program_id,backpack,owner.key)?;
    let (expected,_)=Pubkey::find_program_address(&[MATERIAL_PHYSICS_SEED,global.as_ref()],program_id);
    require_key_eq(physics.key,&expected,E::InvalidMaterialPhysicsPda)?;
    require_key_eq(physics.owner,program_id,E::InvalidMaterialPhysicsData)?;
    let id_bytes=item_id.to_le_bytes();
    let (expected,bump)=Pubkey::find_program_address(&[FORGED_ITEM_SEED,owner.key.as_ref(),&id_bytes],program_id);
    require_key_eq(item.key,&expected,E::InvalidForgedItemPda)?;
    if item.owner==program_id {return Err(E::ForgedItemAlreadyInitialized.into());}
    if item.owner!=&system_program::ID || item.data_len()!=0 {return Err(E::InvalidSystemAccount.into());}
    let clock=Clock::get()?;let (hash,_)=verified_forge_design(AXE_CODE)?;
    { let physics_data=physics.try_borrow_data()?;let table=MaterialPhysicsTableView::new(&physics_data)?;
      let mut data=backpack.try_borrow_mut_data()?;
      craft_record(&mut data,owner.key,&payload[9..],item_id,item.key,hash,&table,clock.slot)?;
    }
    super::create_forged_item_pda(owner,item,system,program_id,item_id,bump)?;
    ForgedItemAccount::pack(&mut item.try_borrow_mut_data()?,&ForgedItemInitArgs{bump,item_id,creator:owner.key,origin_backpack:backpack.key,design_hash:hash,code:AXE_CODE,created_slot:clock.slot,created_at:clock.unix_timestamp})
}

pub fn craft_record(data:&mut [u8],owner:&Pubkey,indexes:&[u8],item_id:u64,item:&Pubkey,hash:u32,physics:&MaterialPhysicsTableView,slot:u64)->ProgramResult {
    BackpackAccount::validate_owner(data,owner)?;
    if indexes.len()<2 || indexes.len()>3 || item_id==0 || *item==Pubkey::default() {return Err(E::InvalidInstruction.into());}
    let mut quantities=[0u32;BACKPACK_MAX_CAPACITY as usize];let fuels=[false;BACKPACK_MAX_CAPACITY as usize];
    let mut stone=1u32;let mut twigs=2u32;let mut volume=0u64;let mut mass=0u64;
    for index in indexes {
        if *index as usize>=quantities.len() || quantities[*index as usize]!=0 {return Err(E::InvalidResourceIndex.into());}
        let record=BackpackAccount::slot_at(data,*index)?;
        if record.kind!=BACKPACK_SLOT_KIND_BLOCK {return Err(E::InvalidForgingMaterial.into());}
        let left=match record.block_id()? {6=>&mut stone,TWIG_BLOCK_ID=>&mut twigs,_=>return Err(E::InvalidForgingMaterial.into())};
        let used=record.quantity.min(*left);
        if used==0 {return Err(E::InvalidForgingMaterial.into());}
        *left-=used;quantities[*index as usize]=used;
        let consumed=if used==record.quantity {record.volume_mm3 as u64}else{(record.volume_mm3 as u64)*used as u64/record.quantity as u64};
        volume+=consumed;mass+=physics.block_mass_grams(record.block_id()?,consumed as u32)? as u64;
    }
    if stone!=0 || twigs!=0 || volume==0 || mass==0 {return Err(E::InvalidForgingMaterial.into());}
    BackpackAccount::consume_smelting_resources(data,owner,&quantities,&fuels,physics,slot)?;
    let mut output=BackpackSlotRecord {kind:BACKPACK_SLOT_KIND_ITEM,category:BACKPACK_ITEM_CATEGORY_FORGED,quantity:1,item_code:BACKPACK_FORGED_ITEM_CODE,item_id,item_pda:*item,volume_mm3:volume.min(750000) as u32,durability_current:AXE_DURABILITY,durability_max:AXE_DURABILITY,grade:1,item_level:1,quality_bps:10000,metadata:hash,..BackpackSlotRecord::default()};
    output.set_mass_grams(mass.min(1100) as u32)?;
    BackpackAccount::append_item(data,owner,&output,slot)
}
