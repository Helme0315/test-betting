use anchor_lang::{prelude::*, solana_program::clock};
use anchor_lang::solana_program::{program::invoke, program::invoke_signed, system_instruction };
use std::mem::size_of;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub mod error;
pub mod rand;
use crate::{error::Error};

#[program]
pub mod test_betting {
    use super::*;

    /// admin init bet global info such as fee(10% team) and treasury account for fee
    pub fn initialize(
        ctx: Context<Initialize>,
        bet_fee: u16
    ) -> Result<()> {
        // get bump for bet_info_account
        let bump = *ctx.bumps.get("bet_info_account").unwrap();

        // init account and info for betting such as admin key, treasury key, fee(10% for team)
        ctx.accounts.bet_info_account.admin_account = ctx.accounts.admin_account.key();
        ctx.accounts.bet_info_account.treasury_account = ctx.accounts.treasury_account.key();
        ctx.accounts.bet_info_account.bet_fee = bet_fee; // 10% = 100, 100% = 1000
        ctx.accounts.bet_info_account.bump = bump;
        Ok(())
    }

    /// admin update bet global info such as fee(10% team) and treasury account for fee
    pub fn update(
        ctx: Context<Update>,
        bet_fee: u16
    ) -> Result<()> {
        // update account info for betting such as admin key, treasury key, fee(10% for team)
        ctx.accounts.bet_info_account.treasury_account = ctx.accounts.treasury_account.key();
        ctx.accounts.bet_info_account.bet_fee = bet_fee;
        Ok(())
    }

    /// admin create a new bet
    pub fn initialize_bet(
        ctx: Context<InitializeBet>,
        start_time: u32,
        end_time: u32
    ) -> Result<()> {
        // get bump for bet_detail_account
        let bump = *ctx.bumps.get("bet_detail_account").unwrap();

        // check current time is bigger than end time of bet or less than start time of bet
        let cur_time = clock::Clock::get().unwrap().unix_timestamp as u32;
        if cur_time > end_time {
            return Err(error!(Error::WrongBettingPeriod));
        }
        if start_time > end_time {
            return Err(error!(Error::WrongBettingPeriod));
        }
        
        // set details for bet such as admin key, start time and end time of bet
        ctx.accounts.bet_detail_account.admin_account = ctx.accounts.admin_account.key();
        ctx.accounts.bet_detail_account.total_bet_amount = 0;
        ctx.accounts.bet_detail_account.start_time = start_time;
        ctx.accounts.bet_detail_account.end_time = end_time;
        ctx.accounts.bet_detail_account.total_l_amount = 0;
        ctx.accounts.bet_detail_account.total_r_amount = 0;
        ctx.accounts.bet_detail_account.winner_result = 0;
        ctx.accounts.bet_detail_account.is_close = false;
        ctx.accounts.bet_detail_account.bump = bump;
        Ok(())
    }

    // admin finalize bet
    pub fn finialize_bet(
        ctx: Context<FinailizeBet>,
        start_time: u32,
    ) -> Result<()> {
        // check bet is already closed or not
        if ctx.accounts.bet_detail_account.is_close {
            return Err(error!(Error::AlreadyEnd));
        }

        // get current time
        let cur_time = clock::Clock::get().unwrap().unix_timestamp as u32;
        // set end time of bet before end time of bet 
        if cur_time < ctx.accounts.bet_detail_account.end_time {
            ctx.accounts.bet_detail_account.end_time = cur_time;
        }
        
        ctx.accounts.bet_detail_account.is_close = true;

        // get random number to get result; 1: winner who bet R, 0: winnder who bet L
        let bet_result = rand::generate(cur_time, ctx.accounts.bet_detail_account.total_bet_amount);
        ctx.accounts.bet_detail_account.winner_result = bet_result;

        // get the team fee(10%) of lossess bet amount
        let mut team_fee = 0;
        if bet_result == 1 {
            team_fee = ctx.accounts.bet_detail_account.total_l_amount.checked_mul(ctx.accounts.bet_info_account.bet_fee as u64).unwrap().checked_div(1000 as u64).unwrap();
        } else{
            team_fee = ctx.accounts.bet_detail_account.total_r_amount.checked_mul(ctx.accounts.bet_info_account.bet_fee as u64).unwrap().checked_div(1000 as u64).unwrap();
        }

        let (_pda, bump_seed) = Pubkey::find_program_address(&[b"escrow".as_ref()], ctx.program_id);

        // transfer fee of bet to treasury account which is setted by admin
        invoke_signed(
            &system_instruction::transfer(
                ctx.accounts.escrow_account.key,
                ctx.accounts.treasury_account.key,
                team_fee,
            ),
            &[
                ctx.accounts.escrow_account.to_account_info().clone(),
                ctx.accounts.treasury_account.to_account_info().clone(),
                ctx.accounts.system_program.to_account_info().clone(),
            ],
            &[&[b"escrow", &[bump_seed]]],
        )?;

        Ok(())
    }

    pub fn init_user_bet(
        ctx: Context<InitUserBet>,
        start_time: u32,
    ) -> Result<()> {
        if ctx.accounts.bet_detail_account.is_close {
            return Err(error!(Error::AlreadyEnd));
        }

        // get current time
        let cur_time = clock::Clock::get().unwrap().unix_timestamp as u32;
        // check bet is already started or not
        if cur_time < ctx.accounts.bet_detail_account.start_time {
            return Err(error!(Error::NoStart));
        }
        // check bet is ended or not
        if cur_time > ctx.accounts.bet_detail_account.end_time {
            return Err(error!(Error::AlreadyEnd));
        }

        let bump = *ctx.bumps.get("user_bet_detail_account").unwrap();

        ctx.accounts.user_bet_detail_account.user_account = ctx.accounts.user_account.key();
        ctx.accounts.user_bet_detail_account.bet_l_amount = 0;
        ctx.accounts.user_bet_detail_account.bet_r_amount = 0;
        ctx.accounts.user_bet_detail_account.is_claim = false;
        ctx.accounts.user_bet_detail_account.bet_id = start_time;
        ctx.accounts.user_bet_detail_account.bump = bump;

        Ok(())
    }

    pub fn user_bet(
        ctx: Context<UserBet>,
        bet_amount: u64,
        start_time: u32,
        bet_type: bool, // true: R, false: L
    ) -> Result<()> {
        if ctx.accounts.bet_detail_account.is_close {
            return Err(error!(Error::AlreadyEnd));
        }
        // get current time
        let cur_time = clock::Clock::get().unwrap().unix_timestamp as u32;
        // check bet is already started or not
        if cur_time < ctx.accounts.bet_detail_account.start_time {
            return Err(error!(Error::NoStart));
        }
        // check bet is ended or not
        if cur_time > ctx.accounts.bet_detail_account.end_time {
            return Err(error!(Error::AlreadyEnd));
        }

        // check enough balance for bet amount
        if **ctx.accounts.user_account.lamports.borrow() < bet_amount {
            return Err(error!(Error::NoEnoughSol));
        }
        
        // transfer bet amount to escrow account
        invoke(
            &system_instruction::transfer(
                ctx.accounts.user_account.key,
                ctx.accounts.escrow_account.key,
                bet_amount
            ),
            &[
                ctx.accounts.user_account.to_account_info().clone(),
                ctx.accounts.escrow_account.to_account_info().clone(),
                ctx.accounts.system_program.to_account_info().clone(),
            ],
        )?;
        
        // calculate total bet amount of this betting game
        ctx.accounts.bet_detail_account.total_bet_amount = ctx.accounts.bet_detail_account.total_bet_amount.checked_add(bet_amount).unwrap();

        // calculate R or L bet amount based on user choice
        if bet_type {
            ctx.accounts.bet_detail_account.total_r_amount = ctx.accounts.bet_detail_account.total_r_amount.checked_add(bet_amount).unwrap();
            ctx.accounts.user_bet_detail_account.bet_r_amount = ctx.accounts.user_bet_detail_account.bet_r_amount.checked_add(bet_amount).unwrap();
        } else {
            ctx.accounts.bet_detail_account.total_l_amount = ctx.accounts.bet_detail_account.total_l_amount.checked_add(bet_amount).unwrap();
            ctx.accounts.user_bet_detail_account.bet_l_amount = ctx.accounts.user_bet_detail_account.bet_l_amount.checked_add(bet_amount).unwrap();
        }
        
        Ok(())
    }

    // winner claim bet amount and reward
    pub fn claim_reward(
        ctx: Context<ClaimReward>,
        start_time: u32,
    ) -> Result<()> {
        // check bet is closed or not
        if !ctx.accounts.bet_detail_account.is_close {
            return Err(error!(Error::NoClose));
        }

        // check user claimed reward or not
        if ctx.accounts.user_bet_detail_account.is_claim {
            return Err(error!(Error::AlreadyRewrdClaim));
        }

        let mut user_claim_amount = 0 as u64;

        // calculate the reward of user
        if ctx.accounts.bet_detail_account.winner_result == 1 {
            // get user bet amount for R
            user_claim_amount = user_claim_amount.checked_add(ctx.accounts.user_bet_detail_account.bet_r_amount).unwrap() as u64;

            // calculate totoal winner reward for L bet amount without fee of team
            let winner_reward = ctx.accounts.bet_detail_account.total_l_amount.checked_mul((1000 as u16 - ctx.accounts.bet_info_account.bet_fee) as u64).unwrap().checked_div(1000 as u64).unwrap() as u64;

            // calcuate user reward based on how much deposit on this bet
            let user_reward = (winner_reward as u128).checked_mul(ctx.accounts.user_bet_detail_account.bet_l_amount as u128).unwrap().checked_mul(100 as u128).unwrap().checked_div(ctx.accounts.bet_detail_account.total_l_amount as u128).unwrap() as u64;
            user_claim_amount = user_claim_amount.checked_add(user_reward).unwrap();
        } else {
            user_claim_amount = user_claim_amount.checked_add(ctx.accounts.user_bet_detail_account.bet_l_amount).unwrap() as u64;

            let winner_reward = ctx.accounts.bet_detail_account.total_r_amount.checked_mul((1000 as u16 - ctx.accounts.bet_info_account.bet_fee) as u64).unwrap().checked_div(1000 as u64).unwrap() as u64;

            let user_reward = (winner_reward as u128).checked_mul(ctx.accounts.user_bet_detail_account.bet_r_amount as u128).unwrap().checked_mul(100 as u128).unwrap().checked_div(ctx.accounts.bet_detail_account.total_r_amount as u128).unwrap() as u64;
            user_claim_amount = user_claim_amount.checked_add(user_reward).unwrap();
        }

        // check enough balance for bet amount
        if **ctx.accounts.escrow_account.lamports.borrow() < user_claim_amount {
            return Err(error!(Error::NoEnoughSol));
        }

        let (_pda, bump_seed) = Pubkey::find_program_address(&[b"escrow".as_ref()], ctx.program_id);

        // transfer fee of bet to treasury account which is setted by admin
        invoke_signed(
            &system_instruction::transfer(
                ctx.accounts.escrow_account.key,
                ctx.accounts.user_account.key,
                user_claim_amount,
            ),
            &[
                ctx.accounts.escrow_account.to_account_info().clone(),
                ctx.accounts.user_account.to_account_info().clone(),
                ctx.accounts.system_program.to_account_info().clone(),
            ],
            &[&[b"escrow", &[bump_seed]]],
        )?;

        ctx.accounts.user_bet_detail_account.is_claim = true;
        Ok(())
    }

}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin_account: Signer<'info>,
    #[account(
        init,
        seeds = [
            b"bet-info".as_ref()
        ],
        bump,
        payer = admin_account,
        space = 8 + size_of::<BetInfo>(),
    )]
    pub bet_info_account: Box<Account<'info, BetInfo>>,
    /// CHECK:: safe account
    pub treasury_account: UncheckedAccount<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Update<'info> {
    pub admin_account: Signer<'info>,
    #[account(
        mut,
        seeds = [
            b"bet-info".as_ref()
        ],
        bump = bet_info_account.bump,
        constraint = bet_info_account.admin_account == admin_account.key() @ Error::AccessDenied
    )]
    pub bet_info_account: Box<Account<'info, BetInfo>>,
    /// CHECK:: safe account
    pub treasury_account: UncheckedAccount<'info>,
}

#[derive(Accounts)]
#[instruction(start_time: u32)]
pub struct InitializeBet<'info> {
    #[account(mut)]
    pub admin_account: Signer<'info>,
    #[account(
        init,
        seeds = [
            b"bet-detail".as_ref(),
            &start_time.to_le_bytes(),
        ],
        bump,
        payer = admin_account,
        space = 8 + size_of::<BetDetails>(),
    )]
    pub bet_detail_account: Box<Account<'info, BetDetails>>,
    #[account(
        seeds = [
            b"bet-info".as_ref()
        ],
        bump = bet_info_account.bump,
        constraint = bet_info_account.admin_account == admin_account.key() @ Error::AccessDenied
    )]
    pub bet_info_account: Box<Account<'info, BetInfo>>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(start_time: u32)]
pub struct FinailizeBet<'info> {
    pub admin_account: Signer<'info>,
    #[account(
        mut,
        seeds = [
            b"bet-detail".as_ref(),
            &start_time.to_le_bytes(),
        ],
        bump = bet_detail_account.bump,
    )]
    pub bet_detail_account: Box<Account<'info, BetDetails>>,
    #[account(
        seeds = [
            b"bet-info".as_ref()
        ],
        bump = bet_info_account.bump,
        constraint = bet_info_account.admin_account == admin_account.key() @ Error::AccessDenied
    )]
    pub bet_info_account: Box<Account<'info, BetInfo>>,
    /// CHECK:: safe account
    #[account(
        mut,
        seeds = [
            b"bet-escrow".as_ref(),
        ],
        bump,
    )]
    pub escrow_account: UncheckedAccount<'info>,
    /// CHECK:: safe account
    #[account(
        mut,
        constraint = bet_info_account.treasury_account == treasury_account.key() @ Error::WrongTreasury
    )]
    pub treasury_account: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(start_time: u64)]
pub struct InitUserBet<'info> {
    #[account(mut)]
    pub user_account: Signer<'info>,
    #[account(
        init,
        seeds = [
            b"user-bet".as_ref(),
            &start_time.to_le_bytes(),
            user_account.key().as_ref()
        ],
        bump,
        payer = user_account,
        space = 8 + size_of::<UserBetDetails>(),
    )]
    pub user_bet_detail_account: Box<Account<'info, UserBetDetails>>,
    #[account(
        seeds = [
            b"bet-detail".as_ref(),
            &start_time.to_le_bytes(),
        ],
        bump = bet_detail_account.bump,
    )]
    pub bet_detail_account: Box<Account<'info, BetDetails>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(start_time: u64)]
pub struct UserBet<'info> {
    #[account(mut)]
    pub user_account: Signer<'info>,
    #[account(
        mut,
        seeds = [
            b"bet-detail".as_ref(),
            &start_time.to_le_bytes(),
        ],
        bump = bet_detail_account.bump,
    )]
    pub bet_detail_account: Box<Account<'info, BetDetails>>,
    /// CHECK:: safe account
    #[account(
        mut,
        seeds = [
            b"bet-escrow".as_ref(),
        ],
        bump,
    )]
    pub escrow_account: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [
            b"user-bet".as_ref(),
            &start_time.to_le_bytes(),
            user_account.key().as_ref()
        ],
        bump = user_bet_detail_account.bump,
        constraint = user_bet_detail_account.user_account == user_account.key() @ Error::AccessDenied
    )]
    pub user_bet_detail_account: Box<Account<'info, UserBetDetails>>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(start_time: u64)]
pub struct ClaimReward<'info> {
    #[account(mut)]
    pub user_account: Signer<'info>,
    #[account(
        seeds = [
            b"bet-detail".as_ref(),
            &start_time.to_le_bytes(),
        ],
        bump = bet_detail_account.bump,
    )]
    pub bet_detail_account: Box<Account<'info, BetDetails>>,
    /// CHECK:: safe account
    #[account(
        mut,
        seeds = [
            b"bet-escrow".as_ref(),
        ],
        bump,
    )]
    pub escrow_account: UncheckedAccount<'info>,
    #[account(
        seeds = [
            b"user-bet".as_ref(),
            &start_time.to_le_bytes(),
            user_account.key().as_ref()
        ],
        bump = user_bet_detail_account.bump,
        constraint = user_bet_detail_account.user_account == user_account.key() @ Error::AccessDenied
    )]
    pub user_bet_detail_account: Box<Account<'info, UserBetDetails>>,
    #[account(
        seeds = [
            b"bet-info".as_ref()
        ],
        bump = bet_info_account.bump,
    )]
    pub bet_info_account: Box<Account<'info, BetInfo>>,
    pub system_program: Program<'info, System>,
}

#[account]
#[repr(C)]
pub struct BetInfo {
    pub admin_account: Pubkey,
    pub treasury_account: Pubkey,
    pub bet_fee: u16,
    pub bump: u8,
}

#[account]
#[repr(C)]
pub struct BetDetails {
    pub admin_account: Pubkey,
    pub total_bet_amount: u64,
    pub total_l_amount: u64,
    pub total_r_amount: u64,
    pub start_time: u32,
    pub end_time: u32,
    pub winner_result: u8, // 1: R, 0: L
    pub is_close: bool,
    pub bump: u8,
}

#[repr(C)]
#[account]
pub struct UserBetDetails {
    pub user_account: Pubkey,
    pub bet_l_amount: u64,
    pub bet_r_amount: u64,
    pub bet_id: u32,
    pub is_claim: bool, // true: claimed, false: not claimed for winning reward
    pub bump: u8
}