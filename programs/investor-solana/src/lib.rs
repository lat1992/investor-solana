#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use pump::program::Pump;

declare_id!("VjPs7ZKRwVnQ5XpJxKEuYwWNbHYb9RzbmBrWG9Xbk9r");

#[program]
pub mod investor_solana {
    use std::u64;

    use super::*;

    pub fn create_session(ctx: Context<CreateSession>, basic_deposit_amount: u64) -> Result<()> {
        let session = &mut ctx.accounts.session;
        session.basic_deposit_amount = basic_deposit_amount;
        session.total_deposit = session.to_account_info().lamports();
        session.is_active = true;
        Ok(())
    }

    pub fn close_session(ctx: Context<CloseSession>, winner_token: Pubkey) -> Result<()> {
        let session = &mut ctx.accounts.session;
        session.winner_token = winner_token;
        session.is_active = false;

        pump::cpi::buy(
            CpiContext::new(
                ctx.accounts.pump_program.to_account_info(),
                pump::cpi::accounts::Buy {
                    global: ctx.accounts.global.to_account_info(),
                    fee_recipient: ctx.accounts.fee_recipient.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    bonding_curve: ctx.accounts.bonding_curve.to_account_info(),
                    associated_bonding_curve: ctx
                        .accounts
                        .associated_bonding_curve
                        .to_account_info(),
                    associated_user: ctx.accounts.associated_user.to_account_info(),
                    user: ctx.accounts.signer.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                    rent: ctx.accounts.rent.to_account_info(),
                    event_authority: ctx.accounts.event_authority.to_account_info(),
                    program: ctx.accounts.pump_program.to_account_info(),
                },
            ),
            u64::MAX,
            session.total_deposit,
        )?;
        Ok(())
    }

    pub fn buy_ticket(ctx: Context<BuyTicket>, token: Pubkey) -> Result<()> {
        let ticket = &mut ctx.accounts.ticket;
        let session = &mut ctx.accounts.session;
        if session.is_active {
            ticket.session = session.key();
            ticket.token = token;
            ticket.is_claim = false;
            let ix = anchor_lang::solana_program::system_instruction::transfer(
                &ctx.accounts.owner.key(),
                &session.key(),
                session.basic_deposit_amount,
            );
            anchor_lang::solana_program::program::invoke(
                &ix,
                &[
                    ctx.accounts.owner.to_account_info(),
                    session.to_account_info(),
                ],
            )?;
            Ok(())
        } else {
            Err(ErrorCode::SessionInactive.into())
        }
    }

    pub fn claim_ticket(ctx: Context<ClaimTicket>) -> Result<()> {
        let ticket = &mut ctx.accounts.ticket;
        let session = &mut ctx.accounts.session;
        if !session.is_active {
            let amount =
                session.winner_token_amount * session.basic_deposit_amount / session.total_deposit;
            let ix = anchor_lang::solana_program::system_instruction::transfer(
                &session.key(),
                &ctx.accounts.owner.key(),
                amount,
            );
            anchor_lang::solana_program::program::invoke(
                &ix,
                &[
                    session.to_account_info(),
                    ctx.accounts.owner.to_account_info(),
                ],
            )?;
            ticket.is_claim = true;

            Ok(())
        } else {
            Err(ErrorCode::SessionIsActive.into())
        }
    }
}

#[derive(Accounts)]
pub struct CreateSession<'info> {
    #[account(init, payer = signer,
        space = 8 + // discriminator
                32 + // target_token (Pubkey)
                8 + // basic_deposit_amount (u64)
                8 + // total_deposit (u64)
                1 // is_active (bool)
    )]
    pub session: Account<'info, Session>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CloseSession<'info> {
    #[account(mut)]
    pub session: Account<'info, Session>,
    pub pump_program: Program<'info, Pump>,
    /// CHECK: This account is checked in the pump program CPI call
    #[account(mut)]
    pub global: AccountInfo<'info>,
    /// CHECK: This account is checked in the pump program CPI call
    #[account(mut)]
    pub fee_recipient: AccountInfo<'info>,
    pub mint: Account<'info, Mint>,
    /// CHECK: This account is checked in the pump program CPI call
    #[account(mut)]
    pub bonding_curve: AccountInfo<'info>,
    #[account(
            mut,
            associated_token::mint = mint,
            associated_token::authority = bonding_curve
        )]
    pub associated_bonding_curve: Account<'info, TokenAccount>,
    #[account(
            mut,
            associated_token::mint = mint,
            associated_token::authority = signer
        )]
    pub associated_user: Account<'info, TokenAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
    /// CHECK: This account is checked in the pump program CPI call
    pub event_authority: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct BuyTicket<'info> {
    #[account(init, payer = owner,
        space = 8 + // discriminator
                32 + // session (Pubkey)
                32 + // token (Pubkey)
                8 + // deposit (u64)
                1 // is_claim (bool)
    )]
    pub ticket: Account<'info, Ticket>,
    #[account(mut)]
    pub session: Account<'info, Session>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimTicket<'info> {
    #[account(mut)]
    pub ticket: Account<'info, Ticket>,
    #[account(mut)]
    pub session: Account<'info, Session>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct Session {
    pub winner_token: Pubkey,
    pub winner_token_amount: u64,
    pub basic_deposit_amount: u64,
    pub total_deposit: u64,
    pub is_active: bool,
}

#[account]
pub struct Ticket {
    pub session: Pubkey,
    pub token: Pubkey,
    pub deposit: u64,
    pub is_claim: bool,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Unauthorized action")]
    Unauthorized,
    #[msg("Session is not active")]
    SessionInactive,
    #[msg("Session is active")]
    SessionIsActive,
    #[msg("Invalid session ID")]
    InvalidSession,
}
