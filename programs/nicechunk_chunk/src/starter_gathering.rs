//! Narrow, authenticated bare-hand entry points; no Equipment account or durability CPI.
use super::*;
pub const PICKED_SEED:&[u8]=b"surface-picked-v1";
pub const PICKED_LEN:usize=48;
pub const PICKED_MAGIC:&[u8;8]=b"NCKPCK01";

pub fn mine(program_id:&Pubkey,accounts:&[AccountInfo],payload:&[u8])->ProgramResult {
    if accounts.len()!=14 {return Err(NicechunkChunkError::InvalidAccountCount.into());}
    mine_block_with_rewards(program_id,accounts,payload,true)
}

pub fn pick(program_id:&Pubkey,accounts:&[AccountInfo],payload:&[u8])->ProgramResult {
    if accounts.len()!=15 {return Err(NicechunkChunkError::InvalidAccountCount.into());}
    let reward=RewardMineArgs::unpack(payload)?;let args=reward.block;
    let payer=&accounts[0];let profile=&accounts[1];let session=&accounts[2];
    let broken=&accounts[4];let foundation=&accounts[5];let global=&accounts[6];let table=&accounts[8];
    let backpack_program=&accounts[9];let backpack=&accounts[10];let physics=&accounts[11];let system=&accounts[13];let picked=&accounts[14];
    if !picked.is_writable || !broken.is_writable || !backpack.is_writable {return Err(NicechunkChunkError::InvalidWritableAccount.into());}
    let context=validate_player_action(payer,profile,session,global,system)?;let config=&context.config;
    let generated=args.generated_args(config)?;generated.validate(config)?;
    let surface=generated_surface_height(config,args.world_x,args.world_z);
    let surface_block=generated_block_id_at_surface(config,&generated,surface);
    if args.world_y!=surface || args.expected_block_id!=surface_block {return Err(NicechunkChunkError::GeneratedBlockMismatch.into());}
    validate_rule_table_pda(program_id,table.key,global.key,SURFACE_DECORATION_TABLE_SEED,NicechunkChunkError::InvalidSurfaceDecorationTablePda)?;
    require_key_eq(table.owner,program_id,NicechunkChunkError::InvalidSurfaceDecorationTableData)?;
    let decoration=surface_decoration_from_table_at_surface(config,&table.try_borrow_data()?,args.world_x,args.world_z,surface)?
        .filter(|e| (100..=107).contains(&e.decoration_id) && e.flags&SURFACE_DECORATION_FLAG_MINEABLE!=0)
        .ok_or(NicechunkChunkError::UnmineableBlock)?;
    let (cx,cz,lx,lz)=args.chunk_coords(config)?;
    reject_foundation_protected_block(program_id,foundation,global,cx,cz,&context.owner,&args)?;
    validate_backpack(backpack_program,backpack)?;
    let bump=validate_chunk_broken_pda(program_id,broken.key,global.key,cx,cz)?;
    create_or_grow_chunk_broken_if_needed(payer,broken,global,system,program_id,config.min_build_y,cx,cz,bump,false)?;
    {let data=broken.try_borrow_data()?;
      ChunkBrokenState::validate_header(&data,config.min_build_y)?;
      if ChunkBrokenState::contains_packed(&data,pack_broken_coord(lx,surface,lz,config.min_build_y)?)? {return Err(NicechunkChunkError::BlockAlreadyMined.into());}
    }
    let bx=cx.to_le_bytes();let bz=cz.to_le_bytes();
    let (expected,pick_bump)=Pubkey::find_program_address(&[PICKED_SEED,global.key.as_ref(),&bx,&bz],program_id);
    require_key_eq(picked.key,&expected,NicechunkChunkError::InvalidChunkBrokenPda)?;
    let seeds=[PICKED_SEED,global.key.as_ref(),&bx,&bz,&[pick_bump]];
    let created=create_fixed_pda_account(payer,picked,system,program_id,PICKED_LEN,&seeds)?;
    {let mut data=picked.try_borrow_mut_data()?;
      if created {data.fill(0);data[..8].copy_from_slice(PICKED_MAGIC);data[8..12].copy_from_slice(&bx);data[12..16].copy_from_slice(&bz);}
      if data[..8]!=PICKED_MAGIC[..] || data[8..12]!=bx || data[12..16]!=bz {return Err(NicechunkChunkError::InvalidChunkBrokenData.into());}
      let index=(lz as usize)*16+lx as usize;
      if data[16+index/8]&(1<<(index%8))!=0 {return Err(NicechunkChunkError::BlockAlreadyMined.into());}
      data[16+index/8]|=1<<(index%8);
    }
    // Loose surface stones are a distinct inventory resource, never terrain Gravel.
    append_backpack_block_resource(program_id,backpack_program,broken,global,profile,backpack,physics,cx,cz,bump,args.world_x,
        pack_backpack_resource_y(surface.saturating_add(1),state::BLOCK_STONE_FRAGMENT,config.min_build_y),args.world_z,500_000,reward.action_id,false)?;
    let _=decoration; // Rule authentication above is required even though output is normalized.
    Ok(())
}
