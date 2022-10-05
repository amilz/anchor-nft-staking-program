use crate::*;
use anchor_lang::solana_program;

#[derive(Accounts)]
pub struct OpenLootbox<'info> {
  #[account(mut)]
  pub user: Signer<'info>,
  #[account(
        init_if_needed,
        payer = user,
        space = std::mem::size_of::<LootboxPointer>() + 8,
        seeds=["lootbox".as_bytes(), user.key().as_ref()],
        bump
    )]
  pub lootbox_pointer: Account<'info, LootboxPointer>,
  pub system_program: Program<'info, System>,
  pub token_program: Program<'info, Token>,
  // Swap the next two lines out between prod/testing
  // #[account(mut)]
  #[account(
        mut,
        address="6YR1nuLqkk8VC1v42xJaPKvE9X9pnuqVAvthFUSDsMUL".parse::<Pubkey>().unwrap()
    )]
  pub stake_mint: Account<'info, Mint>,
  #[account(
        mut,
        associated_token::mint=stake_mint,
        associated_token::authority=user
    )]
  pub stake_mint_ata: Account<'info, TokenAccount>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  #[account(
        constraint=stake_state.user_pubkey==user.key(),
    )]
  pub stake_state: Account<'info, UserStakeInfo>,

  #[account(
        mut,
        seeds = [
            user.key().as_ref(),
        ],
        bump = state.load()?.bump,
        has_one = vrf @ LootboxError::InvalidVrfAccount
    )]
  pub state: AccountLoader<'info, UserState>,

  // SWITCHBOARD ACCOUNTS
  #[account(mut,
        has_one = escrow
    )]
  pub vrf: AccountLoader<'info, VrfAccountData>,
  #[account(mut,
        has_one = data_buffer
    )]
  pub oracle_queue: AccountLoader<'info, OracleQueueAccountData>,
  /// CHECK:
  #[account(mut,
        constraint =
            oracle_queue.load()?.authority == queue_authority.key()
    )]
  pub queue_authority: UncheckedAccount<'info>,
  /// CHECK
  #[account(mut)]
  pub data_buffer: AccountInfo<'info>,
  #[account(mut)]
  pub permission: AccountLoader<'info, PermissionAccountData>,
  #[account(mut,
        constraint =
            escrow.owner == program_state.key()
            && escrow.mint == program_state.load()?.token_mint
    )]
  pub escrow: Account<'info, TokenAccount>,
  #[account(mut)]
  pub program_state: AccountLoader<'info, SbState>,
  /// CHECK:
  #[account(
        address = *vrf.to_account_info().owner,
        constraint = switchboard_program.executable == true
    )]
  pub switchboard_program: AccountInfo<'info>,

  // PAYER ACCOUNTS
  #[account(mut,
        constraint =
            payer_wallet.owner == user.key()
            && escrow.mint == program_state.load()?.token_mint
    )]
  pub payer_wallet: Account<'info, TokenAccount>,
  // SYSTEM ACCOUNTS
  /// CHECK:
  #[account(address = solana_program::sysvar::recent_blockhashes::ID)]
  pub recent_blockhashes: AccountInfo<'info>,
}

#[derive(Clone)]
pub struct StakingProgram;

impl anchor_lang::Id for StakingProgram {
  fn id() -> Pubkey {
    "3CUC1Enh3GF7X1vE7ixm1Aq7cv1fTqY7UZvnDoz7X9sZ"
      .parse::<Pubkey>()
      .unwrap()
  }
}

impl OpenLootbox<'_> {
  pub fn process_instruction(ctx: &mut Context<Self>, box_number: u64) -> Result<()> {
    let mut loot_box = 10;
    loop {
      if loot_box > box_number {
        return err!(LootboxError::InvalidLootbox);
      }

      if loot_box == box_number {
        require!(
          ctx.accounts.stake_state.total_earned >= box_number,
          LootboxError::InvalidLootbox
        );
        break;
      } else {
        loot_box = loot_box * 2;
      }
    }

    require!(
      !ctx.accounts.lootbox_pointer.is_initialized || ctx.accounts.lootbox_pointer.claimed,
      LootboxError::InvalidLootbox
    );

    token::burn(
      CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Burn {
          mint: ctx.accounts.stake_mint.to_account_info(),
          from: ctx.accounts.stake_mint_ata.to_account_info(),
          authority: ctx.accounts.user.to_account_info(),
        },
      ),
      box_number * u64::pow(10, 2),
    )?;

    let state = ctx.accounts.state.load()?;
    let bump = state.bump.clone();
    let switchboard_state_bump = state.switchboard_state_bump;
    let vrf_permission_bump = state.vrf_permission_bump;
    drop(state);

    let switchboard_program = ctx.accounts.switchboard_program.to_account_info();

    let vrf_request_randomness = VrfRequestRandomness {
      authority: ctx.accounts.state.to_account_info(),
      vrf: ctx.accounts.vrf.to_account_info(),
      oracle_queue: ctx.accounts.oracle_queue.to_account_info(),
      queue_authority: ctx.accounts.queue_authority.to_account_info(),
      data_buffer: ctx.accounts.data_buffer.to_account_info(),
      permission: ctx.accounts.permission.to_account_info(),
      escrow: ctx.accounts.escrow.clone(),
      payer_wallet: ctx.accounts.payer_wallet.clone(),
      payer_authority: ctx.accounts.user.to_account_info(),
      recent_blockhashes: ctx.accounts.recent_blockhashes.to_account_info(),
      program_state: ctx.accounts.program_state.to_account_info(),
      token_program: ctx.accounts.token_program.to_account_info(),
    };

    let payer = ctx.accounts.user.key();
    let state_seeds: &[&[&[u8]]] = &[&[payer.as_ref(), &[bump]]];

    msg!("requesting randomness");
    vrf_request_randomness.invoke_signed(
      switchboard_program,
      switchboard_state_bump,
      vrf_permission_bump,
      state_seeds,
    )?;

    let mut state = ctx.accounts.state.load_mut()?;
    state.result = 0;
    state.redeemable = true;

    msg!("randomness requested successfully");

    ctx.accounts.lootbox_pointer.claimed = false;
    ctx.accounts.lootbox_pointer.is_initialized = true;

    Ok(())
  }
}