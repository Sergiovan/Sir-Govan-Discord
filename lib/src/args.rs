use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct SirgovanArgs {
	#[command(subcommand)]
	pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
	/// Run a tournament
	Tournament(TournamentArgs),
}

#[derive(Args)]
pub struct TournamentArgs {
	#[command(subcommand)]
	pub command: TournamentCommand,
}

#[derive(Subcommand)]
pub enum TournamentCommand {
	/// Start creating a tournament
	///
	/// Stores all the message metadata, creates the file skeletons.
	Create(TournamentCreate),

	/// Finalize creating a tournament
	///
	/// Verifies the filled-in skeleton files are in order to begin running a tournament
	Verify(TournamentVerify),

	/// Post a round of a tournament
	///
	/// Creates and posts embeds for each battle in a tournament
	PostRound(TournamentPostRound),

	/// Verify all reactions are in place
	///
	/// Verifies no reactions were missed (Discord can be finnicky with this)
	VerifyRound(TournamentVerifyRound),

	/// Count all reactions and finalize a round
	///
	/// Counts all votes on all the entries and records the results
	FinishRound(TournamentFinishRound),

	/// Removes a round from the channel
	///
	/// Deletes all messages that are entries for a tournament round from the tournament channel
	CleanRound(TournamentCleanRound),

	/// Terminates a tournament and announces a winner
	///
	/// Checks the final entry, does tie resolution and officially announces a winner for the whole tournament
	Finish(TournamentFinish),
}

#[derive(Args)]
pub struct TournamentCreate {
	/// ID of the channel to get the messages from
	pub pin_channel: u64,

	/// ID of the channel where the tournament is held
	pub tournament_channel: u64,

	/// First (oldest; inclusive) message to consider
	pub message_first: u64,

	/// Last (newest; inclusive) message to consider
	pub message_last: u64,

	/// Name of the tournament
	pub tournament_name: String,

	/// Emoji to use as pin
	pub pin_emoji: String,
}

#[derive(Args)]
pub struct TournamentVerify {
	/// Name of the tournament to verify
	pub tournament_name: String,
}

#[derive(Args)]
pub struct TournamentPostRound {
	/// Name of the tournament to post a round for
	pub tournament_name: String,

	/// Number of the round to post
	pub round: u64,

	/// If entries should be reduced to 2^(floor(log n)) after this round
	#[arg(long)]
	pub reduce: bool,
}

#[derive(Args)]
pub struct TournamentVerifyRound {
	/// Name of the tournament to verify a round of
	pub tournament_name: String,

	/// Number of the round to verify
	pub round: u64,
}

#[derive(Args)]
pub struct TournamentFinishRound {
	/// Name of the tournament to finish a round of
	pub tournament_name: String,

	/// Number of the round to finish
	pub round: u64,
}

#[derive(Args)]
pub struct TournamentCleanRound {
	/// Name of the tournament to clean up a round of
	pub tournament_name: String,

	/// Number of the round to clean up
	pub round: u64,
}

#[derive(Args)]
pub struct TournamentFinish {
	/// Name of the tournament to finish
	pub tournament_name: String,

	/// Number of the last round
	pub round: u64,
}
