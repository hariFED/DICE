// Pulse — Streaming VRF dice-roller example
//
// Demonstrates the passive subscriber pattern for DICE streaming VRF:
//
//   1. Player calls `play` with a 1..=6 guess.
//   2. The DICE `RandomnessFeed` PDA is passed as readonly in the same TX.
//   3. This program reads `feed.current_randomness` directly from the
//      account buffer — no CPI, no callback, no second transaction.
//   4. The dice roll is derived as `(randomness[0] % 6) + 1` and the game
//      is settled atomically inside the player's transaction.
//
// Unlike coin-toss, there is no request/commit/reveal round. The feed
// PDA is continuously fresh via the coordinator's feed-crank loop, so
// every `play` TX sees the most recently published value.

use anchor_lang::prelude::*;

declare_id!("J1THpEwf5kYMG25CjeKH2Nfr1oJWMQHW8SzxbvnVwy8t");

/// DICE program ID — the VRF oracle whose RandomnessFeed we subscribe to.
const DICE_PROGRAM_ID: &str = "78Qv6cyKkRZN2YngiLSSBCe2iyRc6jgtCs3incCaMRcv";

/// SHA-256("account:RandomnessFeed")[0..8] — anchor account discriminator.
const FEED_DISCRIMINATOR: [u8; 8] = [171, 75, 107, 39, 47, 205, 68, 158];

/// Offset of `current_randomness` in the RandomnessFeed account layout.
///   disc(8) + authority(32) + coordinator(32) + bound_channel(32)
///   + feed_index(2) + name(32) + publish_interval_slots(4) + status(1) = 143
const FEED_CURRENT_RANDOMNESS_OFFSET: usize = 143;

/// Offset of `current_sequence` (8 bytes, little-endian u64).
const FEED_CURRENT_SEQUENCE_OFFSET: usize = 175;

const SEED_HOUSE: &[u8] = b"pulse_house";
const SEED_PLAY:  &[u8] = b"pulse_play";

#[program]
pub mod pulse {
    use super::*;

    /// Create the House PDA. Tracks lifetime stats.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let house = &mut ctx.accounts.house;
        house.authority = ctx.accounts.authority.key();
        house.total_plays = 0;
        house.total_wins = 0;
        house.bump = ctx.bumps.house;
        msg!("pulse house initialized");
        Ok(())
    }

    /// Roll the dice using the current DICE feed value.
    ///
    /// `guess` must be 1..=6. The outcome is derived from the feed's
    /// `current_randomness[0] % 6 + 1`. A player-scoped `Play` PDA
    /// records the result for audit.
    ///
    /// The feed account is passed as readonly. We verify:
    ///   - account is owned by the DICE program
    ///   - account discriminator matches RandomnessFeed
    ///   - feed.current_sequence > 0 (genesis → no published value yet)
    pub fn play(ctx: Context<Play>, guess: u8, play_nonce: u64) -> Result<()> {
        require!(guess >= 1 && guess <= 6, PulseError::InvalidGuess);

        let feed_ai = &ctx.accounts.dice_feed;

        // Verify the feed account is owned by the DICE program.
        let expected_owner: Pubkey = DICE_PROGRAM_ID.parse().unwrap();
        require!(
            *feed_ai.owner == expected_owner,
            PulseError::FeedWrongOwner
        );

        let data = feed_ai.try_borrow_data()?;
        require!(data.len() >= FEED_CURRENT_SEQUENCE_OFFSET + 8, PulseError::FeedTooSmall);
        require!(
            &data[0..8] == FEED_DISCRIMINATOR.as_ref(),
            PulseError::FeedWrongDiscriminator
        );

        // current_sequence = 0 means genesis — no value has been published.
        let seq_bytes: [u8; 8] = data[FEED_CURRENT_SEQUENCE_OFFSET..FEED_CURRENT_SEQUENCE_OFFSET + 8]
            .try_into()
            .unwrap();
        let current_sequence = u64::from_le_bytes(seq_bytes);
        require!(current_sequence > 0, PulseError::FeedGenesis);

        // Read the 32-byte current_randomness.
        let mut randomness = [0u8; 32];
        randomness.copy_from_slice(
            &data[FEED_CURRENT_RANDOMNESS_OFFSET..FEED_CURRENT_RANDOMNESS_OFFSET + 32],
        );
        drop(data);

        // Derive dice roll 1..=6 from the first byte.
        let roll = (randomness[0] % 6) + 1;
        let won = roll == guess;

        // Persist the play record.
        let play = &mut ctx.accounts.play_record;
        play.player = ctx.accounts.player.key();
        play.guess = guess;
        play.roll = roll;
        play.won = won;
        play.feed_sequence = current_sequence;
        play.randomness = randomness;
        play.play_nonce = play_nonce;
        play.bump = ctx.bumps.play_record;

        // Update house stats.
        let house = &mut ctx.accounts.house;
        house.total_plays = house.total_plays.saturating_add(1);
        if won {
            house.total_wins = house.total_wins.saturating_add(1);
        }

        msg!(
            "pulse play: guess={}, roll={}, {} (feed_seq={})",
            guess,
            roll,
            if won { "WIN" } else { "LOSE" },
            current_sequence
        );
        Ok(())
    }
}

// ─── Accounts ────────────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + House::LEN,
        seeds = [SEED_HOUSE],
        bump,
    )]
    pub house: Account<'info, House>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(guess: u8, play_nonce: u64)]
pub struct Play<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_HOUSE],
        bump = house.bump,
    )]
    pub house: Account<'info, House>,

    #[account(
        init,
        payer = player,
        space = 8 + PlayRecord::LEN,
        seeds = [SEED_PLAY, player.key().as_ref(), &play_nonce.to_le_bytes()],
        bump,
    )]
    pub play_record: Account<'info, PlayRecord>,

    /// CHECK: Validated inside handler — must be owned by the DICE program
    /// and have the RandomnessFeed discriminator. We only read bytes.
    pub dice_feed: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

// ─── State ───────────────────────────────────────────────────────────────────

#[account]
pub struct House {
    pub authority: Pubkey,
    pub total_plays: u64,
    pub total_wins: u64,
    pub bump: u8,
}

impl House {
    const LEN: usize = 32 + 8 + 8 + 1; // 49
}

#[account]
pub struct PlayRecord {
    pub player: Pubkey,
    pub guess: u8,
    pub roll: u8,
    pub won: bool,
    pub feed_sequence: u64,
    pub randomness: [u8; 32],
    pub play_nonce: u64,
    pub bump: u8,
}

impl PlayRecord {
    const LEN: usize = 32 + 1 + 1 + 1 + 8 + 32 + 8 + 1; // 84
}

// ─── Errors ──────────────────────────────────────────────────────────────────

#[error_code]
pub enum PulseError {
    #[msg("Guess must be 1..=6")]
    InvalidGuess,
    #[msg("Feed account is not owned by the DICE program")]
    FeedWrongOwner,
    #[msg("Feed account is too small to be a RandomnessFeed")]
    FeedTooSmall,
    #[msg("Feed account discriminator does not match RandomnessFeed")]
    FeedWrongDiscriminator,
    #[msg("Feed has never been published — current_sequence == 0")]
    FeedGenesis,
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::hash::hashv;

    #[test]
    fn feed_discriminator_matches_expected() {
        let expected = hashv(&[b"account:RandomnessFeed"]);
        let actual: [u8; 8] = expected.to_bytes()[..8].try_into().unwrap();
        assert_eq!(
            FEED_DISCRIMINATOR, actual,
            "hardcoded discriminator must match SHA-256('account:RandomnessFeed')[0..8]"
        );
    }

    #[test]
    fn dice_roll_maps_every_byte_to_1_to_6() {
        let mut hist = [0u32; 6];
        for b in 0u8..=255u8 {
            let roll = (b % 6) + 1;
            hist[(roll - 1) as usize] += 1;
        }
        assert_eq!(hist.iter().sum::<u32>(), 256);
        for c in hist.iter() {
            assert!(*c >= 42 && *c <= 43, "each face should land ~42x over 256 bytes");
        }
    }

    #[test]
    fn play_account_size_is_correct() {
        assert_eq!(PlayRecord::LEN, 84);
    }

    #[test]
    fn house_account_size_is_correct() {
        assert_eq!(House::LEN, 49);
    }
}
