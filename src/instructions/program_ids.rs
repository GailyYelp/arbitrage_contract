use anchor_lang::prelude::*;

use crate::errors::ArbitrageError;
use crate::state::Protocol;

pub const ALDRIN_V2_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("CURVGoZn8zycx6FXwwevgBTB2gVvdbGTEpvMJDbgs2t4");
pub const ALDRIN_V1_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("AMM55ShdkoGRB5jVYPjWziwk8m5MpwyDgsMWHaMSQWH6");

pub const RAYDIUM_CPMM_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");
pub const RAYDIUM_CPMM_PROGRAM_ID_DEVNET: Pubkey =
    anchor_lang::pubkey!("CPMDWBwJDtYax9qW7AyRuVC19Cc4L4Vcy4n2BHAbHkCW");
pub const RAYDIUM_CLMM_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK");
pub const RAYDIUM_CLMM_PROGRAM_ID_DEVNET: Pubkey =
    anchor_lang::pubkey!("devi51mZmdwUJGU9hjN27vEz64Gps7uUefqxg27EAtH");
pub const BYREAL_CLMM_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("REALQqNEomY6cQGZJUGwywTBD2UmDT32rZcNnfxQ5N2");
pub const PANCAKE_SWAP_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("HpNfyc2Saw7RKkQd8nEL4khUcuPhQ7WwY1B2qjx8jxFq");
pub const STABBLE_CLMM_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("6dMXqGZ3ga2dikrYS9ovDXgHGh5RUsb2RTUj6hrQXhk6");
pub const STABBLE_STABLE_SWAP_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("swapNyd8XiQwJ6ianp9snpu4brUqFxadzvHebnAXjJZ");
pub const STABBLE_WEIGHTED_SWAP_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("swapFpHZwjELNnjvThjajtiVmkz3yPQEHjLtka2fwHW");
pub const GAMMA_SWAP_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("GAMMA7meSFWaBXF25oSUgmGRwaW6sCMFLmBNiMSdbHVT");
pub const FUSIONAMM_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("fUSioN9YKKSa3CUC2YUc4tPkHJ5Y6XW1yz8y6F7qWz9");
pub const DERIVERSE_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("DRVSpZ2YUYYKgZP8XtLhAGtT1zYSCKzeHfb4DgRnrgqD");
pub const DERIVERSE_PROGRAM_ID_DEVNET: Pubkey =
    anchor_lang::pubkey!("hSuxfshizdWKiWCVBPhrLBq1yuwLPrGnfmii3JUn613");
pub const CARROT_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("CarrotwivhMpDnm27EHmRLeQ683Z1PufuqEmBZvD282s");
pub const HYLO_EXCHANGE_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("HYEXCHtHkBagdStcJCp3xbbb9B7sdMdWXFNj6mdsG4hn");
pub const HYLO_EARN_POOL_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("HysTabVUfmQBFcmzu1ctRd1Y1fxd66RBpboy1bmtDSQQ");
pub const JUPITER_LEND_EARN_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("jup3YeL8QhtSx1e253b2FDvsMNC87fDrgQZivbrndc9");
pub const JUPITER_LEND_EARN_PROGRAM_ID_DEVNET: Pubkey =
    anchor_lang::pubkey!("7tjE28izRUjzmxC1QNXnNwcc4N82CNYCexf3k8mw67s3");
pub const JUPITER_LEND_LIQUIDITY_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("jupeiUmn818Jg1ekPURTpr4mFo29p46vygyykFJ3wZC");
pub const JUPITER_LEND_LIQUIDITY_PROGRAM_ID_DEVNET: Pubkey =
    anchor_lang::pubkey!("5uDkCoM96pwGYhAUucvCzLfm5UcjVRuxz6gH81RnRBmL");
pub const JUPITER_LEND_REWARDS_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("jup7TthsMgcR9Y3L277b8Eo9uboVSmu1utkuXHNUKar");
pub const JUPITER_LEND_REWARDS_PROGRAM_ID_DEVNET: Pubkey =
    anchor_lang::pubkey!("68LHLkpgjAvo6Lgd9FT6KYEX4FWn1911EohSXxHYMFjc");
pub const M_SWAP_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("MSwapi3WhNKMUGm9YrxGhypgUEt7wYQH3ZgG32XoWzH");
pub const GAVEL_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("srAMMzfVHVAtgSJc8iH6CfKzuWuUTzLHVCE81QU1rgi");
pub const OMNIPAIR_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("omnixgS8fnqHfCcTGKWj6JtKjzpJZ1Y5y9pyFkQDkYE");
pub const SCALE_AMM_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("SCALEwAvEK5gtkdHiFzXfPgtk2YwJxPDzaV3aDmR7tA");
pub const SCALE_VMM_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("SCALEWoRSpVZpMRqHEcDfNvBh3nUSe34jDr9r689gLa");
pub const VIRTUALS_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("5U3EU2ubXtK84QcRjWVmYt9RaDyA8gKxdUrPFXmZyaki");
pub const TRENDS_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("CURVEmPpijXDTNdqrA9PGP1io2rkgiVXH26xdXVGLLfz");
pub const RAYDIUM_STABLE_SWAP_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("5quBtoiQqxF9Jv6KYKctB59NT3gtJD2Y65kdnB1Uev3h");
pub const RAYDIUM_STABLE_SWAP_PROGRAM_ID_DEVNET: Pubkey =
    anchor_lang::pubkey!("DRayDdXc1NZQ9C3hRWmoSf8zK4iapgMnjdNZWrfwsP8m");
pub const WOOFI_SWAP_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("WooFif76YGRNjk1pA8wCsN67aQsD9f9iLsz4NcJ1AVb");
pub const SOLFI_V2_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("SV2EYYJyRz2YhfXwXnhNAevDEui5Q6yrfyo13WtupPF");
pub const SOLFI_V1_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe");
pub const FLUXBEAM_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("FLUXubRmkEi2q6K3Y9kBPg9248ggaZVsoSFhtJHSrm1X");
pub const DEXLAB_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("DSwpgjMvXhtGn6BsbqmacdBZyfLj6jSWf3HJpdJtmg6N");
pub const LEMMINGSFI_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("BQEJZUB4CzoT6UhRffoCkqCyqQNrCPCSGHcPEmsdbEsX");
pub const HADRON_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("HADRoNbLovyqhCsocfYQYB7QdfCAAinN9HTePvBCVDQ8");
pub const BISONFI_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("BiSoNHVpsVZW2F7rx2eQ59yQwKxzU5NvBcmKshCSUypi");
pub const VOLTR_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("vVoLTRjQmtFpiYoegx285Ze4gsLJ8ZxgFKVcuvmG1a8");
pub const ONE_DEX_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("DEXYosS6oEGvk8uCDayvwEZz4qEyDJRf9nFgYCaqPMTm");
pub const HUMA_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("8dWTgQukmAefBAp7nF8kaA1vtZrnR34Zhdmm6Fi24esy");
pub const SOLAYER_ENDOAVS_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("endoLNCKTqDn8gSVnN2hDdpgACUPWHZTwoYnnMybpAT");
pub const CREMA_CLMM_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("CLMM9tUoggJu2wagPkkqs9eFG4BWhVBZWkP1qv3Sp7tR");
pub const SANCTUM_ROUTER_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("stkitrT1Uoy18Dk1fTrgPw8W6MVzoCfYoAFT4MLsmhq");
pub const MOONIT_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("MoonCVVNZFSYkqNXP6bxHLPL6QQJiMagDL3qcqUQTrG");
pub const BOOP_FUN_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("boop8hVGQGqehUK2iVEMEnMrL5RbjywRzHKBmBE7ry4");
pub const HEAVEN_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("HEAVENoP2qxoeuF8Dj2oT1GHEnu49U5mJYkdeC8BAX2o");
pub const SAROS_DLMM_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("1qbkdrr3z4ryLA7pZykqxvxWPoeifcVKo6ZG9CfkvVE");
pub const SAROS_DLMM_PROGRAM_ID_DEVNET: Pubkey =
    anchor_lang::pubkey!("EZoLi7fVCWjns7ukzjggSeDpG2GEGJbGs3MTRxAE29d4");
pub const PERENA_NUMERAIRE_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("NUMERUNsFCP3kuNmWZuXtm1AaQCPj9uw6Guv2Ekoi5P");
pub const PERENA_STAR_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("save8RQVPMWNTzU18t3GBvBkN9hT7jsGjiCQ28FpD9H");
pub const METADAO_FUTARCHY_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("FUTARELBfJfQ8RDGhg1wdhddq1odMAJUePHFuBYfUxKq");
pub const SERUM_V3_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin");
pub const SANCTUM_INFINITY_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("5ocnV1qiCgaQR8Jb8xWnVbApfaygJ8tNoZfgPwsgx9kx");
pub const HUMIDIFI_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("9H6tua7jkLhdm3w8BvgpTn5LZNU7g4ZynDmCiNN3q6Rp");
pub const OBRIC_V2_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("obriQD1zbpyLz95G5n7nJe6a4DPjpFwa5XYPoNm113y");
pub const TESSERA_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("TessVdML9pBGgG9yGks7o4HewRaXVAMuoVj4x83GLQH");
pub const GOONFI_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("goonERTdGsjnkZqWuVjs73BZ3Pb9qoCUdBUL17BnS5j");
pub const GOONFI_V2_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("goonuddtQRrWqqn5nFyczVKaie28f3kDkHWkHtURSLE");
pub const WHALESTREET_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("FW6zUqn4iKRaeopwwhwsquTY6ABWLLgjxtrC3VPnaWBf");
pub const BINARYFI_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("B72M6nyCLFgWiJtAN4naUTminMiTmyGcEqQHXwVeRdht");
pub const XORCA_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("StaKE6XNKVVhG8Qu9hDJBqCW3eRe7MDGLz17nJZetLT");
pub const KIPSELI_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("3TK9D8aoBFYjYZtKCjciPrVrRStsnvo7KmpcJqDavpaU");
pub const RIPTIDE_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("riptK81hDxhe5pW5jSzSM9iRA8azgEgLJ4dXkPtBS7j");
pub const METRIC_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("Bvs46DPFxiFE6YHxLDLD6QAUcmy51FyRVPZJusPxLk3j");
pub const TAURUSFI_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("9VX8EKBg6vM6tA68xaDsPkbrx26XConZjkQmhVApUptc");
pub const SCORCH_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("SCoRcH8c2dpjvcJD6FiPbCSQyQgu3PcUAWj2Xxx3mqn");
pub const SCORCH_ORACLE_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("ojh19ojaKduoJZuaJADhcVGp4xt1TcdAvZmpVsCorch");
pub const RAYDIUM_POOL_V4_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
pub const RAYDIUM_POOL_V4_PROGRAM_ID_DEVNET: Pubkey =
    anchor_lang::pubkey!("DRaya7Kj3aMWQSy19kSjvmuwq9docCHofyP9kanQGaav");
pub const RAYDIUM_LAUNCHPAD_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj");
pub const RAYDIUM_LAUNCHPAD_PROGRAM_ID_DEVNET: Pubkey =
    anchor_lang::pubkey!("DRay6fNdQ5J82H7xV6uq2aV3mNrUZ1J4PgSKsWgptcm6");
pub const PUMPFUN_SWAP_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
pub const PUMPFUN_AMM_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA");
pub const ORCA_WHIRLPOOL_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
pub const HELIUM_TREASURY_MANAGEMENT_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("treaf4wWBBty3fHdyBpo35Mz84M8k3heKXmjmi9vFt5");
pub const HELIUM_CIRCUIT_BREAKER_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("circAbx64bbsscPbQzZAUvuXpHqrCe6fLMzc2uKXz9g");
pub const SABER_ADD_DECIMALS_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("DecZY86MU5Gj7kppfUCEmd4LbXXuyZH1yHaP2NTqdiZB");
pub const ORCA_TOKEN_SWAP_V2_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP");
pub const ORCA_TOKEN_SWAP_V2_PROGRAM_ID_DEVNET: Pubkey =
    anchor_lang::pubkey!("3xQ8SWv2GaFXXpHZNqkXsdxq5DZciHBz6ZFoPPfbFd7U");
pub const ORCA_TOKEN_SWAP_V1_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1");
pub const SAROS_SWAP_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("SSwapUtytfBdBn1b9NUGG6foMVPtcWgpRU32HToDUZr");
pub const SPL_TOKEN_SWAP_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("SwaPpA9LAaLfeLi3a68M4DjnLqgtticKg6CnyNwgAC8");
pub const DOOAR_SWAP_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("Dooar9JkhdZ7J3LHN3A7YCuoGRUggXhQaG4kijfLGU2j");
pub const PENGUIN_SWAP_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("PSwapMdSai8tjrEXcxFeQth87xC4rRsa4VA5mhGhXkP");
pub const SENCHA_SWAP_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("SCHAtsf8mbjyjiv4LkhLKutTf6JnZAbdJKFkXQNMFHZ");
pub const SABER_STABLE_SWAP_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("SSwpkEEcbUqx4vtoEByFjSkhKdCT862DNVb52nZg1UZ");
pub const MERCURIAL_STABLE_SWAP_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("MERLuDFBMmsHnsBPZw2sDQZHvXFMwp8EdjudcU2HKky");
pub const INVARIANT_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("HyaB3W9q6XdA5xwpU4XnSZV94htfmbmqJXZcEbRaJutt");
pub const INVARIANT_PROGRAM_ID_DEVNET: Pubkey =
    anchor_lang::pubkey!("D8Xd5VFXJeANivc4LXEzYqiE8q2CGVbjym5JiynPCP6J");
pub const BONK_SWAP_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("BSwp6bEBihVLdqJRKGgzjcGLHkcTuzmSo1TQkHepzH8p");
pub const BONK_SWAP_STATE: Pubkey =
    anchor_lang::pubkey!("2QWN6WjrJ3RAk51ecxLxaLPfFCYLAnmWJwJ1oKA92CRD");
pub const BONK_SWAP_PROGRAM_AUTHORITY: Pubkey =
    anchor_lang::pubkey!("8NyaPDJeC2eaBGpkRpZKnD9S448AZGgjSvumFe92DRK2");
pub const BONK_SWAP_REFERRER: Pubkey =
    anchor_lang::pubkey!("BUX7s2ef2htTGb2KKoPHWkmzxPj4nTWMWRgs5CSbQxf9");
pub const GUACSWAP_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("Gswppe6ERWKpUTXvRPfXdzHhiCyJvLadVvXGfdpBqcE1");
pub const CROPPER_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("H8W3ctz92svYg6mkn1UtGfu2aQr2fnUFHM1RhScEtQDt");
pub const MANIFEST_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("MNFSTqtC93rEfYHB6hF82sKdZpUDFWkViLByLd1k1Ms");
pub const OPENBOOK_V2_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("opnb2LAfJYbRMAHHvqjCwQxanZn7ReEHp1k81EohpZb");
pub const PHOENIX_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY");
pub const LIFINITY_AMM_V2_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("2wT8Yq49kHgDzXuPxZSaeLaH1qbmGXtEyPy64bL7aD3c");
pub const LIFINITY_AMM_V1_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("EewxydAPCCVuNEyrVN68PuSYdQ7wKn27V9Gjeoi8dy3S");
pub const PHOENIX_LOG_AUTHORITY: Pubkey =
    anchor_lang::pubkey!("7aDTsspkQNGKmrexAN7FLx9oxU3iPczSSvHNggyuqYkR");
pub const METEORA_DAMM_V1_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB");
pub const METEORA_DAMM_V2_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG");
pub const METEORA_DBC_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN");
pub const METEORA_DLMM_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo");
pub const METEORA_VAULT_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("24Uqj9JCLxUeoC3hGfh5W3s9FM9uCHDS2SG3LYwBpyTi");
pub const MEMO_PROGRAM_V2_ID: Pubkey =
    anchor_lang::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");

pub const RAYDIUM_CPMM_AUTHORITY_ID: Pubkey =
    anchor_lang::pubkey!("GpMZbSM2GgvTKHJirzeGfMFoaZ8UR2X7F4v8vHTvxFbL");
pub const RAYDIUM_CPMM_AUTHORITY_ID_DEVNET: Pubkey =
    anchor_lang::pubkey!("7rQ1QFNosMkUCuh7Z7fPbTHvh73b68sQYdirycEzJVuw");
pub const RAYDIUM_LAUNCHPAD_AUTHORITY: Pubkey =
    anchor_lang::pubkey!("WLHv2UAZm6z4KyaaELi5pjdbJh6RESMva1Rnn8pJVVh");
pub const RAYDIUM_LAUNCHPAD_AUTHORITY_DEVNET: Pubkey =
    anchor_lang::pubkey!("5xqNaZXX5eUi4p5HU4oz9i5QnwRNT2y6oN7yyn4qENeq");
pub const RAYDIUM_LAUNCHPAD_EVENT_AUTHORITY: Pubkey =
    anchor_lang::pubkey!("2DPAtwB8L12vrMRExbLuyGnC7n2J5LNoZQSejeQGpwkr");
pub const RAYDIUM_LAUNCHPAD_EVENT_AUTHORITY_DEVNET: Pubkey =
    anchor_lang::pubkey!("4uAB7seenFJKPUXqYewAdfra2u6baBgjiXU8x1SC7Ycz");
pub const RAYDIUM_LAUNCHPAD_EVENT_AUTHORITY_SEED: &[u8] = b"__event_authority";
pub const PUMPFUN_SWAP_GLOBAL_ACCOUNT: Pubkey =
    anchor_lang::pubkey!("4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf");
pub const PUMPFUN_SWAP_FEE_RECIPIENT: Pubkey =
    anchor_lang::pubkey!("68yFSZxzLWJXkxxRGydZ63C6mHx1NLEDWmwN9Lb5yySg");
pub const PUMPFUN_SWAP_EVENT_AUTHORITY: Pubkey =
    anchor_lang::pubkey!("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1");
pub const PUMPFUN_AMM_GLOBAL_CONFIG_ACCOUNT: Pubkey =
    anchor_lang::pubkey!("ADyA8hdefvWN2dbGGWFotbzWxrAvLW83WG6QCVXvJKqw");
pub const PUMPFUN_AMM_FEE_RECIPIENT: Pubkey =
    anchor_lang::pubkey!("62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV");
pub const PUMPFUN_AMM_FEE_RECIPIENT_DEVNET: Pubkey =
    anchor_lang::pubkey!("3PAxmkxnM2vHno9amWQCsaaFjYnPGcD87HZGx1ChVjPj");
pub const PUMPFUN_AMM_EVENT_AUTHORITY: Pubkey =
    anchor_lang::pubkey!("GS4CU59F31iL7aR2Q8zVS8DRrcRnXX1yjQ66TqNVQnaR");
pub const PUMPFUN_SWAP_FEE_CONFIG: Pubkey =
    anchor_lang::pubkey!("8Wf5TiAheLUqBrKXeYg2JtAFFMWtKdG2BSFgqUcPVwTt");
pub const PUMPFUN_AMM_FEE_CONFIG: Pubkey =
    anchor_lang::pubkey!("5PHirr8joyTMp9JMm6nW7hNDVyEYdkzDqazxPD7RaTjx");
pub const PUMPFUN_SWAP_FEE_CONFIG_PROGRAM_ID: Pubkey =
    anchor_lang::pubkey!("pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixedAccountExpectation {
    pub index: usize,
    pub key: Pubkey,
}

pub fn validate_step_program_account(protocol: Protocol, program: &AccountInfo) -> Result<()> {
    require!(program.executable, ArbitrageError::InvalidProgramId);
    validate_protocol_program_id(protocol, program.key)
}

pub fn validate_step_fixed_accounts<'info>(
    protocol: Protocol,
    step_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    validate_protocol_fixed_account_keys(protocol, |index| {
        step_accounts.get(index).map(AccountInfo::key)
    })
}

pub fn validate_protocol_program_id(protocol: Protocol, program_id: &Pubkey) -> Result<()> {
    #[cfg(feature = "flex")]
    {
        let _ = protocol;
        require!(
            program_id != &Pubkey::default(),
            ArbitrageError::InvalidProgramId
        );
        Ok(())
    }

    #[cfg(not(feature = "flex"))]
    {
        require!(
            program_id == &expected_protocol_program_id(protocol),
            ArbitrageError::InvalidProgramId
        );
        Ok(())
    }
}

#[cfg(feature = "flex")]
pub fn validate_protocol_fixed_account_keys(
    protocol: Protocol,
    mut key_at: impl FnMut(usize) -> Option<Pubkey>,
) -> Result<()> {
    let _ = protocol;
    let _ = &mut key_at;
    Ok(())
}

#[cfg(not(feature = "flex"))]
pub fn validate_protocol_fixed_account_keys(
    protocol: Protocol,
    mut key_at: impl FnMut(usize) -> Option<Pubkey>,
) -> Result<()> {
    for expected in expected_fixed_accounts(protocol) {
        let actual_key = key_at(expected.index).ok_or(ArbitrageError::InvalidAccountCount)?;
        require_keys_eq!(actual_key, expected.key, ArbitrageError::InvalidAccount);
    }
    Ok(())
}

#[cfg(not(feature = "flex"))]
pub fn expected_protocol_program_id(protocol: Protocol) -> Pubkey {
    match protocol {
        Protocol::RaydiumCPMM => raydium_cpmm_program_id(),
        Protocol::RaydiumCLMM => raydium_clmm_program_id(),
        Protocol::ByrealCLMM => BYREAL_CLMM_PROGRAM_ID,
        Protocol::PancakeSwap => PANCAKE_SWAP_PROGRAM_ID,
        Protocol::StabbleCLMM => STABBLE_CLMM_PROGRAM_ID,
        Protocol::StabbleStableSwap => STABBLE_STABLE_SWAP_PROGRAM_ID,
        Protocol::StabbleWeightedSwap => STABBLE_WEIGHTED_SWAP_PROGRAM_ID,
        Protocol::GammaSwap => GAMMA_SWAP_PROGRAM_ID,
        Protocol::FusionAmm => FUSIONAMM_PROGRAM_ID,
        Protocol::Deriverse => deriverse_program_id(),
        Protocol::Carrot => CARROT_PROGRAM_ID,
        Protocol::HyloExchange => HYLO_EXCHANGE_PROGRAM_ID,
        Protocol::MSwap => M_SWAP_PROGRAM_ID,
        Protocol::RaydiumStableSwap => raydium_stable_swap_program_id(),
        Protocol::WoofiSwap => WOOFI_SWAP_PROGRAM_ID,
        Protocol::RaydiumPoolV4 => raydium_pool_v4_program_id(),
        Protocol::RaydiumLaunchPad => raydium_launchpad_program_id(),
        Protocol::PumpFunSwap => PUMPFUN_SWAP_PROGRAM_ID,
        Protocol::PumpFunAMM => PUMPFUN_AMM_PROGRAM_ID,
        Protocol::OrcaWhirlpool => ORCA_WHIRLPOOL_PROGRAM_ID,
        Protocol::OrcaTokenSwapV2 => orca_token_swap_v2_program_id(),
        Protocol::OrcaTokenSwapV1 => ORCA_TOKEN_SWAP_V1_PROGRAM_ID,
        Protocol::SarosSwap => SAROS_SWAP_PROGRAM_ID,
        Protocol::SplTokenSwap => SPL_TOKEN_SWAP_PROGRAM_ID,
        Protocol::DooarSwap => DOOAR_SWAP_PROGRAM_ID,
        Protocol::PenguinSwap => PENGUIN_SWAP_PROGRAM_ID,
        Protocol::SenchaSwap => SENCHA_SWAP_PROGRAM_ID,
        Protocol::SaberStableSwap => SABER_STABLE_SWAP_PROGRAM_ID,
        Protocol::MercurialStableSwap => MERCURIAL_STABLE_SWAP_PROGRAM_ID,
        Protocol::Invariant => invariant_program_id(),
        Protocol::BonkSwap => BONK_SWAP_PROGRAM_ID,
        Protocol::Guacswap => GUACSWAP_PROGRAM_ID,
        Protocol::Cropper => CROPPER_PROGRAM_ID,
        Protocol::Manifest => MANIFEST_PROGRAM_ID,
        Protocol::OpenBookV2 => OPENBOOK_V2_PROGRAM_ID,
        Protocol::AldrinV2 => ALDRIN_V2_PROGRAM_ID,
        Protocol::AldrinV1 => ALDRIN_V1_PROGRAM_ID,
        Protocol::SolFiV2 => SOLFI_V2_PROGRAM_ID,
        Protocol::SolFiV1 => SOLFI_V1_PROGRAM_ID,
        Protocol::FluxBeam => FLUXBEAM_PROGRAM_ID,
        Protocol::Dexlab => DEXLAB_PROGRAM_ID,
        Protocol::LemmingsFi => LEMMINGSFI_PROGRAM_ID,
        Protocol::Hadron => HADRON_PROGRAM_ID,
        Protocol::BisonFi => BISONFI_PROGRAM_ID,
        Protocol::Voltr => VOLTR_PROGRAM_ID,
        Protocol::OneDex => ONE_DEX_PROGRAM_ID,
        Protocol::Huma => HUMA_PROGRAM_ID,
        Protocol::SolayerEndoAvs => SOLAYER_ENDOAVS_PROGRAM_ID,
        Protocol::HyloEarnPool => HYLO_EARN_POOL_PROGRAM_ID,
        Protocol::JupiterLendEarn => jupiter_lend_earn_program_id(),
        Protocol::HeliumTreasuryManagement => HELIUM_TREASURY_MANAGEMENT_PROGRAM_ID,
        Protocol::SaberAddDecimals => SABER_ADD_DECIMALS_PROGRAM_ID,
        Protocol::CremaClmm => CREMA_CLMM_PROGRAM_ID,
        Protocol::SanctumRouter => SANCTUM_ROUTER_PROGRAM_ID,
        Protocol::Moonit => MOONIT_PROGRAM_ID,
        Protocol::BoopFun => BOOP_FUN_PROGRAM_ID,
        Protocol::Heaven => HEAVEN_PROGRAM_ID,
        Protocol::SarosDlmm => saros_dlmm_program_id(),
        Protocol::PerenaNumeraire => PERENA_NUMERAIRE_PROGRAM_ID,
        Protocol::PerenaStar => PERENA_STAR_PROGRAM_ID,
        Protocol::MetaDaoFutarchy => METADAO_FUTARCHY_PROGRAM_ID,
        Protocol::Gavel => GAVEL_PROGRAM_ID,
        Protocol::Omnipair => OMNIPAIR_PROGRAM_ID,
        Protocol::ScaleAmm => SCALE_AMM_PROGRAM_ID,
        Protocol::ScaleVmm => SCALE_VMM_PROGRAM_ID,
        Protocol::Virtuals => VIRTUALS_PROGRAM_ID,
        Protocol::Trends => TRENDS_PROGRAM_ID,
        Protocol::SerumV3 => SERUM_V3_PROGRAM_ID,
        Protocol::SanctumInfinity => SANCTUM_INFINITY_PROGRAM_ID,
        Protocol::HumidiFi => HUMIDIFI_PROGRAM_ID,
        Protocol::ObricV2 => OBRIC_V2_PROGRAM_ID,
        Protocol::Tessera => TESSERA_PROGRAM_ID,
        Protocol::GoonFi => GOONFI_PROGRAM_ID,
        Protocol::GoonFiV2 => GOONFI_V2_PROGRAM_ID,
        Protocol::WhaleStreet => WHALESTREET_PROGRAM_ID,
        Protocol::BinaryFi => BINARYFI_PROGRAM_ID,
        Protocol::XOrca => XORCA_PROGRAM_ID,
        Protocol::Kipseli => KIPSELI_PROGRAM_ID,
        Protocol::Riptide => RIPTIDE_PROGRAM_ID,
        Protocol::Metric => METRIC_PROGRAM_ID,
        Protocol::TaurusFi => TAURUSFI_PROGRAM_ID,
        Protocol::Scorch => SCORCH_PROGRAM_ID,
        Protocol::Phoenix => PHOENIX_PROGRAM_ID,
        Protocol::LifinityAmmV2 => LIFINITY_AMM_V2_PROGRAM_ID,
        Protocol::LifinityAmmV1 => LIFINITY_AMM_V1_PROGRAM_ID,
        Protocol::MeteoraDammV2 => METEORA_DAMM_V2_PROGRAM_ID,
        Protocol::MeteoraDammV1 => METEORA_DAMM_V1_PROGRAM_ID,
        Protocol::MeteoraDbc => METEORA_DBC_PROGRAM_ID,
        Protocol::MeteoraDlmm => METEORA_DLMM_PROGRAM_ID,
    }
}

#[cfg(not(feature = "flex"))]
pub fn expected_fixed_accounts(protocol: Protocol) -> &'static [FixedAccountExpectation] {
    match protocol {
        Protocol::RaydiumCPMM => raydium_cpmm_fixed_accounts(),
        Protocol::RaydiumCLMM => RAYDIUM_CLMM_FIXED_ACCOUNTS,
        Protocol::ByrealCLMM => BYREAL_CLMM_FIXED_ACCOUNTS,
        Protocol::PancakeSwap => PANCAKE_SWAP_FIXED_ACCOUNTS,
        Protocol::StabbleCLMM => STABBLE_CLMM_FIXED_ACCOUNTS,
        Protocol::StabbleStableSwap => NO_FIXED_ACCOUNTS,
        Protocol::StabbleWeightedSwap => NO_FIXED_ACCOUNTS,
        Protocol::GammaSwap => NO_FIXED_ACCOUNTS,
        Protocol::FusionAmm => NO_FIXED_ACCOUNTS,
        Protocol::Deriverse => NO_FIXED_ACCOUNTS,
        Protocol::Carrot => NO_FIXED_ACCOUNTS,
        Protocol::HyloExchange => NO_FIXED_ACCOUNTS,
        Protocol::MSwap => NO_FIXED_ACCOUNTS,
        Protocol::RaydiumStableSwap => NO_FIXED_ACCOUNTS,
        Protocol::WoofiSwap => NO_FIXED_ACCOUNTS,
        Protocol::RaydiumPoolV4 => NO_FIXED_ACCOUNTS,
        Protocol::RaydiumLaunchPad => raydium_launchpad_fixed_accounts(),
        Protocol::PumpFunSwap => PUMPFUN_SWAP_FIXED_ACCOUNTS,
        Protocol::PumpFunAMM => pumpfun_amm_fixed_accounts(),
        Protocol::OrcaWhirlpool => ORCA_WHIRLPOOL_FIXED_ACCOUNTS,
        Protocol::OrcaTokenSwapV2 => NO_FIXED_ACCOUNTS,
        Protocol::OrcaTokenSwapV1 => NO_FIXED_ACCOUNTS,
        Protocol::SarosSwap => NO_FIXED_ACCOUNTS,
        Protocol::SplTokenSwap => NO_FIXED_ACCOUNTS,
        Protocol::DooarSwap => NO_FIXED_ACCOUNTS,
        Protocol::PenguinSwap => NO_FIXED_ACCOUNTS,
        Protocol::SenchaSwap => NO_FIXED_ACCOUNTS,
        Protocol::SaberStableSwap => NO_FIXED_ACCOUNTS,
        Protocol::MercurialStableSwap => NO_FIXED_ACCOUNTS,
        Protocol::Invariant => NO_FIXED_ACCOUNTS,
        Protocol::BonkSwap => NO_FIXED_ACCOUNTS,
        Protocol::Guacswap => NO_FIXED_ACCOUNTS,
        Protocol::Cropper => NO_FIXED_ACCOUNTS,
        Protocol::Manifest => NO_FIXED_ACCOUNTS,
        Protocol::OpenBookV2 => OPENBOOK_V2_FIXED_ACCOUNTS,
        Protocol::AldrinV2 => NO_FIXED_ACCOUNTS,
        Protocol::AldrinV1 => NO_FIXED_ACCOUNTS,
        Protocol::SolFiV2 => NO_FIXED_ACCOUNTS,
        Protocol::SolFiV1 => NO_FIXED_ACCOUNTS,
        Protocol::FluxBeam => NO_FIXED_ACCOUNTS,
        Protocol::Dexlab => NO_FIXED_ACCOUNTS,
        Protocol::LemmingsFi => NO_FIXED_ACCOUNTS,
        Protocol::Hadron => NO_FIXED_ACCOUNTS,
        Protocol::BisonFi => NO_FIXED_ACCOUNTS,
        Protocol::Voltr => NO_FIXED_ACCOUNTS,
        Protocol::OneDex => NO_FIXED_ACCOUNTS,
        Protocol::Huma => NO_FIXED_ACCOUNTS,
        Protocol::SolayerEndoAvs => NO_FIXED_ACCOUNTS,
        Protocol::HyloEarnPool => NO_FIXED_ACCOUNTS,
        Protocol::JupiterLendEarn => NO_FIXED_ACCOUNTS,
        Protocol::HeliumTreasuryManagement => NO_FIXED_ACCOUNTS,
        Protocol::SaberAddDecimals => NO_FIXED_ACCOUNTS,
        Protocol::CremaClmm => NO_FIXED_ACCOUNTS,
        Protocol::SanctumRouter => NO_FIXED_ACCOUNTS,
        Protocol::Moonit => NO_FIXED_ACCOUNTS,
        Protocol::BoopFun => NO_FIXED_ACCOUNTS,
        Protocol::Heaven => NO_FIXED_ACCOUNTS,
        Protocol::SarosDlmm => NO_FIXED_ACCOUNTS,
        Protocol::PerenaNumeraire => NO_FIXED_ACCOUNTS,
        Protocol::PerenaStar => PERENA_STAR_FIXED_ACCOUNTS,
        Protocol::MetaDaoFutarchy => NO_FIXED_ACCOUNTS,
        Protocol::Gavel => NO_FIXED_ACCOUNTS,
        Protocol::Omnipair => NO_FIXED_ACCOUNTS,
        Protocol::ScaleAmm => NO_FIXED_ACCOUNTS,
        Protocol::ScaleVmm => NO_FIXED_ACCOUNTS,
        Protocol::Virtuals => NO_FIXED_ACCOUNTS,
        Protocol::Trends => NO_FIXED_ACCOUNTS,
        Protocol::SerumV3 => NO_FIXED_ACCOUNTS,
        Protocol::SanctumInfinity => SANCTUM_INFINITY_FIXED_ACCOUNTS,
        Protocol::HumidiFi => NO_FIXED_ACCOUNTS,
        Protocol::ObricV2 => NO_FIXED_ACCOUNTS,
        Protocol::Tessera => NO_FIXED_ACCOUNTS,
        Protocol::GoonFi => NO_FIXED_ACCOUNTS,
        Protocol::GoonFiV2 => NO_FIXED_ACCOUNTS,
        Protocol::WhaleStreet => NO_FIXED_ACCOUNTS,
        Protocol::BinaryFi => NO_FIXED_ACCOUNTS,
        Protocol::XOrca => NO_FIXED_ACCOUNTS,
        Protocol::Kipseli => NO_FIXED_ACCOUNTS,
        Protocol::Riptide => NO_FIXED_ACCOUNTS,
        Protocol::Metric => NO_FIXED_ACCOUNTS,
        Protocol::TaurusFi => NO_FIXED_ACCOUNTS,
        Protocol::Scorch => NO_FIXED_ACCOUNTS,
        Protocol::Phoenix => PHOENIX_FIXED_ACCOUNTS,
        Protocol::LifinityAmmV2 => LIFINITY_AMM_V2_FIXED_ACCOUNTS,
        Protocol::LifinityAmmV1 => LIFINITY_AMM_V1_FIXED_ACCOUNTS,
        Protocol::MeteoraDammV2 => NO_FIXED_ACCOUNTS,
        Protocol::MeteoraDammV1 => METEORA_DAMM_V1_FIXED_ACCOUNTS,
        Protocol::MeteoraDbc => NO_FIXED_ACCOUNTS,
        Protocol::MeteoraDlmm => METEORA_DLMM_FIXED_ACCOUNTS,
    }
}

#[cfg(all(feature = "devnet", not(feature = "flex")))]
fn raydium_cpmm_program_id() -> Pubkey {
    RAYDIUM_CPMM_PROGRAM_ID_DEVNET
}

#[cfg(all(not(feature = "flex"), feature = "devnet"))]
fn deriverse_program_id() -> Pubkey {
    DERIVERSE_PROGRAM_ID_DEVNET
}

#[cfg(all(not(feature = "flex"), not(feature = "devnet")))]
fn deriverse_program_id() -> Pubkey {
    DERIVERSE_PROGRAM_ID
}

#[cfg(all(not(feature = "devnet"), not(feature = "flex")))]
fn raydium_cpmm_program_id() -> Pubkey {
    RAYDIUM_CPMM_PROGRAM_ID
}

#[cfg(all(feature = "devnet", not(feature = "flex")))]
fn raydium_clmm_program_id() -> Pubkey {
    RAYDIUM_CLMM_PROGRAM_ID_DEVNET
}

#[cfg(all(not(feature = "devnet"), not(feature = "flex")))]
fn raydium_clmm_program_id() -> Pubkey {
    RAYDIUM_CLMM_PROGRAM_ID
}

#[cfg(all(feature = "devnet", not(feature = "flex")))]
fn raydium_stable_swap_program_id() -> Pubkey {
    RAYDIUM_STABLE_SWAP_PROGRAM_ID_DEVNET
}

#[cfg(all(not(feature = "devnet"), not(feature = "flex")))]
fn raydium_stable_swap_program_id() -> Pubkey {
    RAYDIUM_STABLE_SWAP_PROGRAM_ID
}

#[cfg(all(feature = "devnet", not(feature = "flex")))]
fn raydium_pool_v4_program_id() -> Pubkey {
    RAYDIUM_POOL_V4_PROGRAM_ID_DEVNET
}

#[cfg(all(not(feature = "devnet"), not(feature = "flex")))]
fn raydium_pool_v4_program_id() -> Pubkey {
    RAYDIUM_POOL_V4_PROGRAM_ID
}

#[cfg(all(feature = "devnet", not(feature = "flex")))]
fn raydium_launchpad_program_id() -> Pubkey {
    RAYDIUM_LAUNCHPAD_PROGRAM_ID_DEVNET
}

#[cfg(all(feature = "devnet", not(feature = "flex")))]
fn orca_token_swap_v2_program_id() -> Pubkey {
    ORCA_TOKEN_SWAP_V2_PROGRAM_ID_DEVNET
}

#[cfg(all(feature = "devnet", not(feature = "flex")))]
fn invariant_program_id() -> Pubkey {
    INVARIANT_PROGRAM_ID_DEVNET
}

#[cfg(all(not(feature = "devnet"), not(feature = "flex")))]
fn invariant_program_id() -> Pubkey {
    INVARIANT_PROGRAM_ID
}

#[cfg(all(feature = "devnet", not(feature = "flex")))]
fn saros_dlmm_program_id() -> Pubkey {
    SAROS_DLMM_PROGRAM_ID_DEVNET
}

#[cfg(all(not(feature = "devnet"), not(feature = "flex")))]
fn saros_dlmm_program_id() -> Pubkey {
    SAROS_DLMM_PROGRAM_ID
}

#[cfg(feature = "devnet")]
pub const fn jupiter_lend_earn_program_id() -> Pubkey {
    JUPITER_LEND_EARN_PROGRAM_ID_DEVNET
}

#[cfg(not(feature = "devnet"))]
pub const fn jupiter_lend_earn_program_id() -> Pubkey {
    JUPITER_LEND_EARN_PROGRAM_ID
}

#[cfg(feature = "devnet")]
pub const fn jupiter_lend_liquidity_program_id() -> Pubkey {
    JUPITER_LEND_LIQUIDITY_PROGRAM_ID_DEVNET
}

#[cfg(not(feature = "devnet"))]
pub const fn jupiter_lend_liquidity_program_id() -> Pubkey {
    JUPITER_LEND_LIQUIDITY_PROGRAM_ID
}

#[cfg(feature = "devnet")]
pub const fn jupiter_lend_rewards_program_id() -> Pubkey {
    JUPITER_LEND_REWARDS_PROGRAM_ID_DEVNET
}

#[cfg(not(feature = "devnet"))]
pub const fn jupiter_lend_rewards_program_id() -> Pubkey {
    JUPITER_LEND_REWARDS_PROGRAM_ID
}

#[cfg(all(not(feature = "devnet"), not(feature = "flex")))]
fn orca_token_swap_v2_program_id() -> Pubkey {
    ORCA_TOKEN_SWAP_V2_PROGRAM_ID
}

#[cfg(all(not(feature = "devnet"), not(feature = "flex")))]
fn raydium_launchpad_program_id() -> Pubkey {
    RAYDIUM_LAUNCHPAD_PROGRAM_ID
}

#[cfg(not(feature = "flex"))]
const NO_FIXED_ACCOUNTS: &[FixedAccountExpectation] = &[];

#[cfg(not(feature = "flex"))]
const SANCTUM_INFINITY_FIXED_ACCOUNTS: &[FixedAccountExpectation] = &[
    FixedAccountExpectation {
        index: 1,
        key: crate::protocal::sanctum_infinity::SANCTUM_INFINITY_POOL_STATE,
    },
    FixedAccountExpectation {
        index: 2,
        key: crate::protocal::sanctum_infinity::SANCTUM_INFINITY_LST_STATE_LIST,
    },
];

#[cfg(not(feature = "flex"))]
const PERENA_STAR_FIXED_ACCOUNTS: &[FixedAccountExpectation] = &[
    FixedAccountExpectation {
        index: 1,
        key: crate::protocal::perena_star::PERENA_STAR_BANK_STATE,
    },
    FixedAccountExpectation {
        index: 2,
        key: crate::protocal::perena_star::PERENA_STAR_USDC_VAULT,
    },
];

#[cfg(not(feature = "flex"))]
const PHOENIX_FIXED_ACCOUNTS: &[FixedAccountExpectation] = &[
    FixedAccountExpectation {
        index: 1,
        key: PHOENIX_LOG_AUTHORITY,
    },
    FixedAccountExpectation {
        index: 7,
        key: anchor_spl::token::ID,
    },
];

#[cfg(not(feature = "flex"))]
const OPENBOOK_V2_FIXED_ACCOUNTS: &[FixedAccountExpectation] = &[
    FixedAccountExpectation {
        index: 8,
        key: anchor_spl::token::ID,
    },
    FixedAccountExpectation {
        index: 9,
        key: anchor_lang::system_program::ID,
    },
];

#[cfg(not(feature = "flex"))]
const LIFINITY_AMM_V2_FIXED_ACCOUNTS: &[FixedAccountExpectation] = &[
    FixedAccountExpectation {
        index: 0,
        key: LIFINITY_AMM_V2_PROGRAM_ID,
    },
    FixedAccountExpectation {
        index: 9,
        key: anchor_spl::token::ID,
    },
];

#[cfg(not(feature = "flex"))]
const LIFINITY_AMM_V1_FIXED_ACCOUNTS: &[FixedAccountExpectation] = &[
    FixedAccountExpectation {
        index: 0,
        key: LIFINITY_AMM_V1_PROGRAM_ID,
    },
    FixedAccountExpectation {
        index: 9,
        key: anchor_spl::token::ID,
    },
];

#[cfg(all(feature = "devnet", not(feature = "flex")))]
const RAYDIUM_CPMM_FIXED_ACCOUNTS_DEVNET: &[FixedAccountExpectation] = &[FixedAccountExpectation {
    index: 1,
    key: RAYDIUM_CPMM_AUTHORITY_ID_DEVNET,
}];

#[cfg(all(not(feature = "devnet"), not(feature = "flex")))]
const RAYDIUM_CPMM_FIXED_ACCOUNTS_MAINNET: &[FixedAccountExpectation] =
    &[FixedAccountExpectation {
        index: 1,
        key: RAYDIUM_CPMM_AUTHORITY_ID,
    }];

#[cfg(not(feature = "flex"))]
const RAYDIUM_CLMM_FIXED_ACCOUNTS: &[FixedAccountExpectation] = &[FixedAccountExpectation {
    index: 6,
    key: MEMO_PROGRAM_V2_ID,
}];

#[cfg(not(feature = "flex"))]
const BYREAL_CLMM_FIXED_ACCOUNTS: &[FixedAccountExpectation] = &[FixedAccountExpectation {
    index: 6,
    key: MEMO_PROGRAM_V2_ID,
}];

#[cfg(not(feature = "flex"))]
const PANCAKE_SWAP_FIXED_ACCOUNTS: &[FixedAccountExpectation] = &[FixedAccountExpectation {
    index: 6,
    key: MEMO_PROGRAM_V2_ID,
}];

#[cfg(not(feature = "flex"))]
const STABBLE_CLMM_FIXED_ACCOUNTS: &[FixedAccountExpectation] = &[FixedAccountExpectation {
    index: 6,
    key: MEMO_PROGRAM_V2_ID,
}];

#[cfg(not(feature = "flex"))]
const ORCA_WHIRLPOOL_FIXED_ACCOUNTS: &[FixedAccountExpectation] = &[FixedAccountExpectation {
    index: 3,
    key: MEMO_PROGRAM_V2_ID,
}];

#[cfg(not(feature = "flex"))]
const METEORA_DLMM_FIXED_ACCOUNTS: &[FixedAccountExpectation] = &[FixedAccountExpectation {
    index: 3,
    key: MEMO_PROGRAM_V2_ID,
}];

#[cfg(not(feature = "flex"))]
const METEORA_DAMM_V1_FIXED_ACCOUNTS: &[FixedAccountExpectation] = &[FixedAccountExpectation {
    index: 12,
    key: METEORA_VAULT_PROGRAM_ID,
}];

#[cfg(all(feature = "devnet", not(feature = "flex")))]
const RAYDIUM_LAUNCHPAD_FIXED_ACCOUNTS_DEVNET: &[FixedAccountExpectation] = &[
    FixedAccountExpectation {
        index: 1,
        key: RAYDIUM_LAUNCHPAD_AUTHORITY_DEVNET,
    },
    FixedAccountExpectation {
        index: 7,
        key: RAYDIUM_LAUNCHPAD_EVENT_AUTHORITY_DEVNET,
    },
];

#[cfg(all(not(feature = "devnet"), not(feature = "flex")))]
const RAYDIUM_LAUNCHPAD_FIXED_ACCOUNTS_MAINNET: &[FixedAccountExpectation] = &[
    FixedAccountExpectation {
        index: 1,
        key: RAYDIUM_LAUNCHPAD_AUTHORITY,
    },
    FixedAccountExpectation {
        index: 7,
        key: RAYDIUM_LAUNCHPAD_EVENT_AUTHORITY,
    },
];

#[cfg(not(feature = "flex"))]
const PUMPFUN_SWAP_FIXED_ACCOUNTS: &[FixedAccountExpectation] = &[
    FixedAccountExpectation {
        index: 1,
        key: PUMPFUN_SWAP_GLOBAL_ACCOUNT,
    },
    FixedAccountExpectation {
        index: 2,
        key: PUMPFUN_SWAP_FEE_RECIPIENT,
    },
];

#[cfg(all(feature = "devnet", not(feature = "flex")))]
const PUMPFUN_AMM_FIXED_ACCOUNTS_DEVNET: &[FixedAccountExpectation] = &[
    FixedAccountExpectation {
        index: 2,
        key: PUMPFUN_AMM_GLOBAL_CONFIG_ACCOUNT,
    },
    FixedAccountExpectation {
        index: 5,
        key: PUMPFUN_AMM_FEE_RECIPIENT_DEVNET,
    },
    FixedAccountExpectation {
        index: 7,
        key: PUMPFUN_AMM_EVENT_AUTHORITY,
    },
];

#[cfg(all(not(feature = "devnet"), not(feature = "flex")))]
const PUMPFUN_AMM_FIXED_ACCOUNTS_MAINNET: &[FixedAccountExpectation] = &[
    FixedAccountExpectation {
        index: 2,
        key: PUMPFUN_AMM_GLOBAL_CONFIG_ACCOUNT,
    },
    FixedAccountExpectation {
        index: 5,
        key: PUMPFUN_AMM_FEE_RECIPIENT,
    },
    FixedAccountExpectation {
        index: 7,
        key: PUMPFUN_AMM_EVENT_AUTHORITY,
    },
];

#[cfg(all(feature = "devnet", not(feature = "flex")))]
fn raydium_cpmm_fixed_accounts() -> &'static [FixedAccountExpectation] {
    RAYDIUM_CPMM_FIXED_ACCOUNTS_DEVNET
}

#[cfg(all(not(feature = "devnet"), not(feature = "flex")))]
fn raydium_cpmm_fixed_accounts() -> &'static [FixedAccountExpectation] {
    RAYDIUM_CPMM_FIXED_ACCOUNTS_MAINNET
}

#[cfg(all(feature = "devnet", not(feature = "flex")))]
fn raydium_launchpad_fixed_accounts() -> &'static [FixedAccountExpectation] {
    RAYDIUM_LAUNCHPAD_FIXED_ACCOUNTS_DEVNET
}

#[cfg(all(not(feature = "devnet"), not(feature = "flex")))]
fn raydium_launchpad_fixed_accounts() -> &'static [FixedAccountExpectation] {
    RAYDIUM_LAUNCHPAD_FIXED_ACCOUNTS_MAINNET
}

#[cfg(all(feature = "devnet", not(feature = "flex")))]
fn pumpfun_amm_fixed_accounts() -> &'static [FixedAccountExpectation] {
    PUMPFUN_AMM_FIXED_ACCOUNTS_DEVNET
}

#[cfg(all(not(feature = "devnet"), not(feature = "flex")))]
fn pumpfun_amm_fixed_accounts() -> &'static [FixedAccountExpectation] {
    PUMPFUN_AMM_FIXED_ACCOUNTS_MAINNET
}

#[cfg(test)]
mod tests {
    use super::*;

    const CUSTOM_PROGRAM_ID: Pubkey =
        anchor_lang::pubkey!("BPFLoaderUpgradeab1e11111111111111111111111");

    #[cfg(not(feature = "flex"))]
    fn expected_launchpad_fixed_keys() -> (Pubkey, Pubkey) {
        #[cfg(feature = "devnet")]
        {
            (
                RAYDIUM_LAUNCHPAD_AUTHORITY_DEVNET,
                RAYDIUM_LAUNCHPAD_EVENT_AUTHORITY_DEVNET,
            )
        }
        #[cfg(not(feature = "devnet"))]
        {
            (
                RAYDIUM_LAUNCHPAD_AUTHORITY,
                RAYDIUM_LAUNCHPAD_EVENT_AUTHORITY,
            )
        }
    }

    #[cfg(not(feature = "flex"))]
    #[test]
    fn expected_programs_match_selected_cluster_defaults() {
        #[cfg(not(feature = "devnet"))]
        assert_eq!(
            expected_protocol_program_id(Protocol::RaydiumCPMM),
            RAYDIUM_CPMM_PROGRAM_ID
        );
        #[cfg(feature = "devnet")]
        assert_eq!(
            expected_protocol_program_id(Protocol::RaydiumCPMM),
            RAYDIUM_CPMM_PROGRAM_ID_DEVNET
        );

        #[cfg(not(feature = "devnet"))]
        assert_eq!(
            expected_protocol_program_id(Protocol::RaydiumCLMM),
            RAYDIUM_CLMM_PROGRAM_ID
        );
        #[cfg(feature = "devnet")]
        assert_eq!(
            expected_protocol_program_id(Protocol::RaydiumCLMM),
            RAYDIUM_CLMM_PROGRAM_ID_DEVNET
        );

        #[cfg(not(feature = "devnet"))]
        assert_eq!(
            expected_protocol_program_id(Protocol::RaydiumStableSwap),
            RAYDIUM_STABLE_SWAP_PROGRAM_ID
        );
        #[cfg(feature = "devnet")]
        assert_eq!(
            expected_protocol_program_id(Protocol::RaydiumStableSwap),
            RAYDIUM_STABLE_SWAP_PROGRAM_ID_DEVNET
        );

        #[cfg(not(feature = "devnet"))]
        assert_eq!(
            expected_protocol_program_id(Protocol::RaydiumPoolV4),
            RAYDIUM_POOL_V4_PROGRAM_ID
        );
        #[cfg(feature = "devnet")]
        assert_eq!(
            expected_protocol_program_id(Protocol::RaydiumPoolV4),
            RAYDIUM_POOL_V4_PROGRAM_ID_DEVNET
        );

        #[cfg(not(feature = "devnet"))]
        assert_eq!(
            expected_protocol_program_id(Protocol::RaydiumLaunchPad),
            RAYDIUM_LAUNCHPAD_PROGRAM_ID
        );
        #[cfg(feature = "devnet")]
        assert_eq!(
            expected_protocol_program_id(Protocol::RaydiumLaunchPad),
            RAYDIUM_LAUNCHPAD_PROGRAM_ID_DEVNET
        );
        #[cfg(not(feature = "devnet"))]
        assert_eq!(
            expected_protocol_program_id(Protocol::SarosDlmm),
            SAROS_DLMM_PROGRAM_ID
        );
        #[cfg(feature = "devnet")]
        assert_eq!(
            expected_protocol_program_id(Protocol::SarosDlmm),
            SAROS_DLMM_PROGRAM_ID_DEVNET
        );
        assert_eq!(
            expected_protocol_program_id(Protocol::PumpFunSwap),
            PUMPFUN_SWAP_PROGRAM_ID
        );
        assert_eq!(
            expected_protocol_program_id(Protocol::PumpFunAMM),
            PUMPFUN_AMM_PROGRAM_ID
        );
        assert_eq!(
            expected_protocol_program_id(Protocol::OrcaWhirlpool),
            ORCA_WHIRLPOOL_PROGRAM_ID
        );
        assert_eq!(
            expected_protocol_program_id(Protocol::Cropper),
            CROPPER_PROGRAM_ID
        );
        assert_eq!(
            expected_protocol_program_id(Protocol::Dexlab),
            DEXLAB_PROGRAM_ID
        );
        assert_eq!(
            expected_protocol_program_id(Protocol::LemmingsFi),
            LEMMINGSFI_PROGRAM_ID
        );
        assert_eq!(
            expected_protocol_program_id(Protocol::Hadron),
            HADRON_PROGRAM_ID
        );
        assert_eq!(
            expected_protocol_program_id(Protocol::Voltr),
            VOLTR_PROGRAM_ID
        );
        assert_eq!(
            expected_protocol_program_id(Protocol::MeteoraDammV2),
            METEORA_DAMM_V2_PROGRAM_ID
        );
        assert_eq!(
            expected_protocol_program_id(Protocol::MeteoraDammV1),
            METEORA_DAMM_V1_PROGRAM_ID
        );
        assert_eq!(
            expected_protocol_program_id(Protocol::MeteoraDbc),
            METEORA_DBC_PROGRAM_ID
        );
        assert_eq!(
            expected_protocol_program_id(Protocol::MeteoraDlmm),
            METEORA_DLMM_PROGRAM_ID
        );
        assert_eq!(
            expected_protocol_program_id(Protocol::PancakeSwap),
            PANCAKE_SWAP_PROGRAM_ID
        );
        assert_eq!(
            expected_protocol_program_id(Protocol::StabbleCLMM),
            STABBLE_CLMM_PROGRAM_ID
        );
        assert_eq!(
            expected_protocol_program_id(Protocol::StabbleStableSwap),
            STABBLE_STABLE_SWAP_PROGRAM_ID
        );
        assert_eq!(
            expected_protocol_program_id(Protocol::StabbleWeightedSwap),
            STABBLE_WEIGHTED_SWAP_PROGRAM_ID
        );
        assert_eq!(
            expected_protocol_program_id(Protocol::GammaSwap),
            GAMMA_SWAP_PROGRAM_ID
        );
        assert_eq!(
            expected_protocol_program_id(Protocol::WoofiSwap),
            WOOFI_SWAP_PROGRAM_ID
        );
    }

    #[cfg(not(feature = "flex"))]
    #[test]
    fn protocol_program_validation_rejects_wrong_program_id() {
        let err = validate_protocol_program_id(Protocol::RaydiumCPMM, &CUSTOM_PROGRAM_ID)
            .expect_err("wrong program id should fail");

        assert_eq!(err, ArbitrageError::InvalidProgramId.into());
    }

    #[cfg(not(feature = "flex"))]
    #[test]
    fn protocol_program_validation_accepts_expected_program_id() {
        let expected_program_id = expected_protocol_program_id(Protocol::RaydiumCPMM);

        assert!(validate_protocol_program_id(Protocol::RaydiumCPMM, &expected_program_id).is_ok());
    }

    #[cfg(not(feature = "flex"))]
    #[test]
    fn fixed_account_validation_accepts_expected_keys() {
        let (authority, event_authority) = expected_launchpad_fixed_keys();
        let mut keys = [Pubkey::default(); 8];
        keys[1] = authority;
        keys[7] = event_authority;

        assert!(
            validate_protocol_fixed_account_keys(Protocol::RaydiumLaunchPad, |index| {
                keys.get(index).copied()
            })
            .is_ok()
        );
    }

    #[cfg(not(feature = "flex"))]
    #[test]
    fn fixed_account_validation_rejects_wrong_key() {
        let (authority, _) = expected_launchpad_fixed_keys();
        let mut keys = [Pubkey::default(); 8];
        keys[1] = authority;
        keys[7] = CUSTOM_PROGRAM_ID;

        let err = validate_protocol_fixed_account_keys(Protocol::RaydiumLaunchPad, |index| {
            keys.get(index).copied()
        })
        .expect_err("wrong fixed account should fail");

        assert_eq!(err, ArbitrageError::InvalidAccount.into());
    }

    #[cfg(not(feature = "flex"))]
    #[test]
    fn fixed_account_validation_rejects_missing_key() {
        let (authority, _) = expected_launchpad_fixed_keys();
        let keys = [authority];

        let err = validate_protocol_fixed_account_keys(Protocol::RaydiumLaunchPad, |index| {
            keys.get(index).copied()
        })
        .expect_err("missing fixed account should fail");

        assert_eq!(err, ArbitrageError::InvalidAccountCount.into());
    }

    #[cfg(feature = "flex")]
    #[test]
    fn flex_program_validation_accepts_custom_non_default_program_id() {
        assert!(validate_protocol_program_id(Protocol::RaydiumCPMM, &CUSTOM_PROGRAM_ID).is_ok());
    }

    #[cfg(feature = "flex")]
    #[test]
    fn flex_program_validation_rejects_default_program_id() {
        let err = validate_protocol_program_id(Protocol::RaydiumCPMM, &Pubkey::default())
            .expect_err("default program id should fail");

        assert_eq!(err, ArbitrageError::InvalidProgramId.into());
    }

    #[cfg(feature = "flex")]
    #[test]
    fn flex_fixed_account_validation_skips_canonical_key_checks() {
        let keys = [Pubkey::default()];

        assert!(
            validate_protocol_fixed_account_keys(Protocol::RaydiumLaunchPad, |index| {
                keys.get(index).copied()
            })
            .is_ok()
        );
    }

    #[test]
    fn raydium_launchpad_event_authorities_match_anchor_event_pdas() {
        assert_eq!(
            Pubkey::find_program_address(
                &[RAYDIUM_LAUNCHPAD_EVENT_AUTHORITY_SEED],
                &RAYDIUM_LAUNCHPAD_PROGRAM_ID
            )
            .0,
            RAYDIUM_LAUNCHPAD_EVENT_AUTHORITY
        );
        assert_eq!(
            Pubkey::find_program_address(
                &[RAYDIUM_LAUNCHPAD_EVENT_AUTHORITY_SEED],
                &RAYDIUM_LAUNCHPAD_PROGRAM_ID_DEVNET
            )
            .0,
            RAYDIUM_LAUNCHPAD_EVENT_AUTHORITY_DEVNET
        );
    }
}
