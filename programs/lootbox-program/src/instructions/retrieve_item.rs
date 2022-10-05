use crate::state::*;
use crate::*;

#[derive(Accounts)]
pub struct RetrieveItem<'info> {
  #[account(mut)]
  pub user: Signer<'info>,
  // #[account(
  //       seeds=["lootbox".as_bytes(), user.key().as_ref()],
  //       bump,
  //       constraint=lootbox_pointer.is_initialized
  //   )]
  // pub lootbox_pointer: Account<'info, LootboxPointer>,
  #[account(
        mut,
        constraint=state.load()?.mint==mint.key()
    )]
  pub mint: Account<'info, Mint>,
  #[account(
        mut,
        seeds = [
            user.key().as_ref(),
        ],
        bump = state.load()?.bump
    )]
  pub state: AccountLoader<'info, UserState>,
  #[account(
        init_if_needed,
        payer=user,
        associated_token::mint=mint,
        associated_token::authority=user
    )]
  pub user_gear_ata: Account<'info, TokenAccount>,
  /// CHECK: Mint authority - not used as account
  #[account(
        seeds=["mint".as_bytes()],
        bump
    )]
  pub mint_authority: UncheckedAccount<'info>,
  pub token_program: Program<'info, Token>,
  pub associated_token_program: Program<'info, AssociatedToken>,
  pub system_program: Program<'info, System>,
  pub rent: Sysvar<'info, Rent>,
}

impl RetrieveItem<'_> {
  pub fn process_instruction(ctx: &mut Context<Self>) -> Result<()> {
    let state = &mut ctx.accounts.state.load_mut()?;

    require!(state.redeemable, LootboxError::MintNotReady);
    // require!(
    //   ctx.accounts.lootbox_pointer.mint_is_ready,
    //   LootboxError::MintNotReady
    // );
    // require!(
    //   !ctx.accounts.lootbox_pointer.claimed,
    //   LootboxError::AlreadyClaimed
    // );
    token::mint_to(
      CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        MintTo {
          mint: ctx.accounts.mint.to_account_info(),
          to: ctx.accounts.user_gear_ata.to_account_info(),
          authority: ctx.accounts.mint_authority.to_account_info(),
        },
        &[&[
          b"mint".as_ref(),
          &[*ctx.bumps.get("mint_authority").unwrap()],
        ]],
      ),
      1,
    )?;

    state.redeemable = false;
    // ctx.accounts.lootbox_pointer.claimed = true;
    // ctx.accounts.lootbox_pointer.mint_is_ready = false;

    Ok(())
  }
}
