use anchor_lang::prelude::*;

#[error_code]
pub enum Error {
    #[msg("Access Denied")]
    AccessDenied,

    #[msg("The betting period is wrong")]
    WrongBettingPeriod,

    #[msg("The bet round is already finished")]
    AlreadyEnd,

    #[msg("The bet round is not started yet")]
    NoStart,

    #[msg("User doesn't have enough SOL")]
    NoEnoughSol,

    #[msg("Invalid treasury account")]
    WrongTreasury,

    #[msg("Betting is not closed yet")]
    NoClose,

    #[msg("User already claimed the reward")]
    AlreadyRewrdClaim
}