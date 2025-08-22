use anchor_lang::prelude::*;

#[error_code]
pub enum ArbitrageError {
    #[msg("Invalid arbitrage path")]
    InvalidPath,
    
    #[msg("Path too short: minimum 1 step required")]
    PathTooShort,
    
    #[msg("Path too long: maximum 10 steps allowed")]
    PathTooLong,
    
    #[msg("Invalid amount")]
    InvalidAmount,
    
    #[msg("Missing token account")]
    MissingTokenAccount,
    
    #[msg("Insufficient output amount")]
    InsufficientOutputAmount,
    
    #[msg("Unprofitable trade")]
    UnprofitableTrade,
    
    #[msg("Insufficient accounts provided")]
    InsufficientAccounts,
    
    #[msg("Invalid account index")]
    InvalidAccountIndex,
    
    #[msg("Invalid account type for DEX")]
    InvalidAccountType,
    
    #[msg("Swap execution failed")]
    SwapExecutionFailed,
    
    #[msg("Invalid account count for DEX type")]
    InvalidAccountCount,
    
    #[msg("Account not found in remaining accounts")]
    AccountNotFound,
    
    #[msg("Invalid public key format")]
    InvalidPublicKey,
    
    // DEX交换相关错误
    #[msg("Missing required account")]
    MissingRequiredAccount,
    
    #[msg("Invalid account")]
    InvalidAccount,
    
    #[msg("Math overflow")]
    MathOverflow,
    
    #[msg("Invalid token mint")]
    InvalidTokenMint,
    
    #[msg("Insufficient liquidity")]
    InsufficientLiquidity,
    
    #[msg("Unsupported DEX")]
    UnsupportedDex,
    
    #[msg("Invalid slippage")]
    InvalidSlippage,
    
    #[msg("Slippage too high")]
    SlippageTooHigh,
    
    #[msg("Fee too high")]
    FeeTooHigh,
    
    #[msg("Zero amount out")]
    ZeroAmountOut,
    
    #[msg("Invalid fee amount")]
    InvalidFeeAmount,
    
    #[msg("Insufficient profit")]
    InsufficientProfit,
    
    #[msg("DEX health check failed")]
    DexHealthCheckFailed,
    
    #[msg("Invalid instruction data")]
    InvalidInstructionData,
}