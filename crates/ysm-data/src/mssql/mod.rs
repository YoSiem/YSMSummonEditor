//! MSSQL implementation of [`SummonRepository`] on top of tiberius.
//!
//! The repository owns a single-threaded tokio runtime and one lazily opened
//! connection, so its blocking methods can be called from a plain worker
//! thread. A connection that failed or sat idle is transparently reopened.

mod cells;
mod commit;
mod queries;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use tiberius::{AuthMethod, Client, Config, EncryptionLevel};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};
use ysm_core::commit::{CommitRequest, CommitResult, tree_slot_column};
use ysm_core::model::{
    CreatureEnhance, LevelBonusUsage, RandomGroupUsage, SkillDisplay, SkillTreeEntry, SkillUsage, StatScalingSnapshot,
    StatUsage, SummonListFilter, SummonListItem, TreeUsage,
};
use ysm_core::random_group::RandomSkillGroup;
use ysm_core::value::SkillRecord;

use crate::settings::DbSettings;
use crate::{DataError, Result, SummonRepository};

type SqlClient = Client<Compat<TcpStream>>;
type SqlResult<T> = std::result::Result<T, tiberius::error::Error>;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// A connection unused for this long is checked before being trusted again.
const IDLE_CHECK_AFTER: Duration = Duration::from_secs(30);

pub struct MssqlRepository {
    runtime: Runtime,
    settings: DbSettings,
    client: Option<SqlClient>,
    last_used: Instant,
}

impl MssqlRepository {
    /// Creates the repository. No connection is opened until the first query.
    pub fn new(settings: DbSettings) -> Result<Self> {
        if !settings.is_configured() {
            return Err(DataError::NotConfigured);
        }
        let runtime = tokio::runtime::Builder::new_current_thread().enable_io().enable_time().build()?;
        Ok(Self { runtime, settings, client: None, last_used: Instant::now() })
    }

    /// Runs one database operation, opening the connection first when needed.
    fn run<T>(&mut self, operation: impl AsyncFnOnce(&mut SqlClient) -> SqlResult<T>) -> Result<T> {
        let Self { runtime, settings, client, last_used } = self;
        runtime.block_on(async {
            if let Some(existing) = client.as_mut()
                && last_used.elapsed() > IDLE_CHECK_AFTER
                && !is_alive(existing).await
            {
                *client = None;
            }

            let connection = match client {
                Some(connection) => connection,
                None => client.insert(connect(settings).await?),
            };
            let outcome = operation(connection).await;
            *last_used = Instant::now();

            outcome.map_err(|error| {
                // Errors reported by the server leave the connection usable; anything else does not.
                if !matches!(error, tiberius::error::Error::Server(_)) {
                    *client = None;
                }
                DataError::Query(describe(&error))
            })
        })
    }
}

async fn is_alive(client: &mut SqlClient) -> bool {
    match client.simple_query("SELECT 1;").await {
        Ok(stream) => stream.into_results().await.is_ok(),
        Err(_) => false,
    }
}

async fn connect(settings: &DbSettings) -> Result<SqlClient> {
    let host = normalize_host(&settings.server);
    let mut config = Config::new();
    config.host(&host);
    config.port(settings.effective_port());
    config.database(settings.database.trim());
    config.authentication(AuthMethod::sql_server(settings.user_id.trim(), &settings.password));
    config.application_name("YSM Summon Editor");
    // Same as `Encrypt=False` in the original connection string: only the login is encrypted.
    config.encryption(EncryptionLevel::Off);
    if settings.trust_server_certificate {
        config.trust_cert();
    }

    let address = config.get_addr();
    let tcp = tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(&address))
        .await
        .map_err(|_| DataError::Connection(format!("timed out connecting to {address}")))?
        .map_err(|error| DataError::Connection(format!("{address}: {error}")))?;
    tcp.set_nodelay(true).map_err(|error| DataError::Connection(error.to_string()))?;

    tokio::time::timeout(CONNECT_TIMEOUT, Client::connect(config, tcp.compat_write()))
        .await
        .map_err(|_| DataError::Connection(format!("timed out logging in to {address}")))?
        .map_err(|error| DataError::Connection(describe(&error)))
}

/// Strips the parts of an ADO server name that only make sense to SqlClient.
///
/// The port is always given explicitly, so an instance name is redundant.
fn normalize_host(server: &str) -> String {
    let server = server.trim();
    let server = server.strip_prefix("tcp:").unwrap_or(server);
    let host = server.split(['\\', ',']).next().unwrap_or(server).trim();
    match host {
        "." | "(local)" | "" => "localhost".to_owned(),
        other => other.to_owned(),
    }
}

fn describe(error: &tiberius::error::Error) -> String {
    match error {
        tiberius::error::Error::Server(token) => {
            format!("{} (SQL error {}, line {})", token.message(), token.code(), token.line())
        }
        other => other.to_string(),
    }
}

/// Rejects requests that cannot be expressed in SQL before a transaction is opened.
fn validate(request: &CommitRequest) -> Result<()> {
    match request.trees.iter().find(|t| t.clone_if_shared && tree_slot_column(t.slot).is_none()) {
        Some(tree) => Err(DataError::Invalid(format!("Tree slot must be in range 1..5, got {}.", tree.slot))),
        None => Ok(()),
    }
}

impl SummonRepository for MssqlRepository {
    fn test_connection(&mut self) -> Result<String> {
        self.run(async |c| queries::server_version(c).await)
    }

    fn summon_by_id(&mut self, summon_id: i32) -> Result<Option<SummonListItem>> {
        self.run(async |c| queries::summon_by_id(c, summon_id).await)
    }

    fn summons_by_card_id(&mut self, card_id: i32) -> Result<Vec<SummonListItem>> {
        self.run(async |c| queries::summons_by_card_id(c, card_id).await)
    }

    fn search_summons(&mut self, filter: &SummonListFilter) -> Result<Vec<SummonListItem>> {
        self.run(async |c| queries::search_summons(c, filter).await)
    }

    fn skill_tree_entries(&mut self, skill_tree_id: i32) -> Result<Vec<SkillTreeEntry>> {
        self.run(async |c| queries::skill_tree_entries(c, skill_tree_id).await)
    }

    fn stat_scaling(&mut self, summon_id: i32) -> Result<StatScalingSnapshot> {
        self.run(async |c| queries::stat_scaling(c, summon_id).await)?
            .ok_or_else(|| DataError::NotFound(format!("Summon {summon_id}")))
    }

    fn creature_enhance(&mut self) -> Result<Vec<CreatureEnhance>> {
        self.run(async |c| queries::creature_enhance(c).await)
    }

    fn skill_record(&mut self, skill_id: i32) -> Result<SkillRecord> {
        self.run(async |c| queries::skill_record(c, skill_id).await)?
            .ok_or_else(|| DataError::NotFound(format!("Skill {skill_id}")))
    }

    fn skill_display(&mut self, skill_ids: &[i32]) -> Result<HashMap<i32, SkillDisplay>> {
        if skill_ids.is_empty() {
            return Ok(HashMap::new());
        }
        self.run(async |c| queries::skill_display(c, skill_ids).await)
    }

    fn search_skills(&mut self, text: &str, limit: usize) -> Result<Vec<SkillDisplay>> {
        self.run(async |c| queries::search_skills(c, text, limit).await)
    }

    fn random_groups(&mut self, group_ids: &[i32]) -> Result<HashMap<i32, RandomSkillGroup>> {
        if group_ids.is_empty() {
            return Ok(HashMap::new());
        }
        self.run(async |c| queries::random_groups(c, group_ids).await)
    }

    fn next_random_group_id(&mut self) -> Result<i32> {
        self.run(async |c| queries::next_random_group_id(c).await)
    }

    fn skill_usage(&mut self, skill_ids: &[i32]) -> Result<Vec<SkillUsage>> {
        if skill_ids.is_empty() {
            return Ok(Vec::new());
        }
        self.run(async |c| queries::skill_usage(c, skill_ids).await)
    }

    fn tree_usage(&mut self, skill_tree_ids: &[i32]) -> Result<Vec<TreeUsage>> {
        if skill_tree_ids.is_empty() {
            return Ok(Vec::new());
        }
        self.run(async |c| queries::tree_usage(c, skill_tree_ids).await)
    }

    fn stat_usage(&mut self, stat_ids: &[i32]) -> Result<Vec<StatUsage>> {
        if stat_ids.is_empty() {
            return Ok(Vec::new());
        }
        self.run(async |c| queries::stat_usage(c, stat_ids).await)
    }

    fn level_bonus_usage(&mut self, bonus_ids: &[i32]) -> Result<Vec<LevelBonusUsage>> {
        if bonus_ids.is_empty() {
            return Ok(Vec::new());
        }
        self.run(async |c| queries::level_bonus_usage(c, bonus_ids).await)
    }

    fn random_group_usage(&mut self, group_ids: &[i32]) -> Result<Vec<RandomGroupUsage>> {
        if group_ids.is_empty() {
            return Ok(Vec::new());
        }
        self.run(async |c| queries::random_group_usage(c, group_ids).await)
    }

    fn commit(&mut self, request: &CommitRequest) -> Result<CommitResult> {
        validate(request)?;
        self.run(async |c| commit::commit(c, request).await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ysm_core::commit::TreeCommit;

    #[test]
    fn host_is_normalized_for_a_plain_tcp_connection() {
        assert_eq!(normalize_host(" 10.0.0.5 "), "10.0.0.5");
        assert_eq!(normalize_host("tcp:db.example,1433"), "db.example");
        assert_eq!(normalize_host("HOST\\SQLEXPRESS"), "HOST");
        assert_eq!(normalize_host("."), "localhost");
        assert_eq!(normalize_host("(local)"), "localhost");
    }

    #[test]
    fn unconfigured_settings_are_rejected_without_connecting() {
        assert!(matches!(MssqlRepository::new(DbSettings::default()), Err(DataError::NotConfigured)));
    }

    #[test]
    fn clone_into_an_invalid_slot_is_rejected_up_front() {
        let tree = |slot, clone| TreeCommit {
            summon_id: 1,
            slot,
            original_tree_id: 1,
            entries: Vec::new(),
            clone_if_shared: clone,
        };
        assert!(validate(&CommitRequest { trees: vec![tree(5, true)], ..Default::default() }).is_ok());
        assert!(validate(&CommitRequest { trees: vec![tree(6, false)], ..Default::default() }).is_ok());
        assert!(validate(&CommitRequest { trees: vec![tree(6, true)], ..Default::default() }).is_err());
    }

    /// Smoke test against a real database. Run with
    /// `YSM_MSSQL_TEST_CONNECTION="Server=...;Database=...;User Id=...;Password=..." cargo test -- --ignored`.
    #[test]
    #[ignore = "needs a live MSSQL database"]
    fn live_database_smoke_test() {
        let connection_string =
            std::env::var("YSM_MSSQL_TEST_CONNECTION").expect("set YSM_MSSQL_TEST_CONNECTION to run this test");
        let settings = crate::env_local::parse_connection_string(&connection_string).expect("valid connection string");
        let mut repo = MssqlRepository::new(settings).unwrap();

        println!("server: {}", repo.test_connection().unwrap());
        let summons = repo.search_summons(&SummonListFilter::default()).unwrap();
        assert!(!summons.is_empty());

        let bundle = crate::loader::load_chain(&mut repo, &summons[0]).unwrap();
        println!("chain: {:?}", bundle.chain.summon_ids());
        assert!(repo.next_random_group_id().unwrap() >= 1);
        assert!(!repo.search_skills("", 5).unwrap().is_empty());
    }
}
