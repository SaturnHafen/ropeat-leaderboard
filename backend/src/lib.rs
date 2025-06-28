mod error;
mod helper;
mod r#static;
mod templating;

pub use error::LeaderboardError;

use askama::Template;
use axum::{
    extract::{Path, State},
    http::{
        header::{self, AUTHORIZATION},
        HeaderMap,
    },
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
    Extension, Form, Json, Router,
};
use r#static::{font, icon, robots, style};

use serde_derive::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{
    prelude::FromRow,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    ConnectOptions, SqlitePool,
};
use std::{str::FromStr, sync::Arc};
use templating::{ClaimFormTemplate, ClaimListTemplate, LeaderboardTemplate};
use uuid::Uuid;

use crate::{
    helper::slow_equals,
    r#static::{form_style, script},
};

#[derive(Deserialize, Debug, Clone)]
struct RecievedScore {
    score: i32,
    color: String,
}

#[derive(FromRow, Debug, Clone)]
struct UnclaimedScoreRow {
    id: Uuid,
    score: i32,
    color: String,
}

#[derive(FromRow, Serialize, Deserialize)]
pub struct ScoreRow {
    nickname: String,
    score: i32,
}

#[derive(Serialize, Deserialize)]
pub struct PlacementScoreRow {
    nickname: String,
    score: i32,
    placement: u32,
}

#[derive(Deserialize, Debug)]
struct ClaimScore {
    wants_leaderboard: Option<bool>,
    wants_raffle: Option<bool>,

    nickname: String,

    email: String,
    firstname: String,
    lastname: String,
}

impl From<ClaimScore> for HPIFormSubmission {
    fn from(val: ClaimScore) -> Self {
        HPIFormSubmission {
            firstname: val.firstname,
            lastname: val.lastname,
            email: val.email,
        }
    }
}

#[derive(Clone)]
struct LeaderboardConfig<'a> {
    form_submit_url: &'a str,
    base_url: &'a str,
    token: &'a str,
}

#[derive(Debug, Clone, Serialize)]
struct HPIFormSubmission {
    firstname: String,
    lastname: String,
    email: String,
}

struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new() -> Result<Self, LeaderboardError> {
        let db_options = SqliteConnectOptions::from_str(":memory:")
            .map_err(LeaderboardError::DatabaseSetup)?
            .create_if_missing(true)
            .disable_statement_logging()
            .to_owned();

        let pool = SqlitePoolOptions::new()
            .connect_with(db_options)
            .await
            .map_err(LeaderboardError::DatabaseSetup)?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS UnclaimedScores (
                id BLOB(16) PRIMARY KEY,
                score INTEGER NOT NULL,
                color TEXT NOT NULL
            );",
        )
        .execute(&pool)
        .await
        .map_err(LeaderboardError::DatabaseSetup)?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS Scores (
                id INTEGER PRIMARY KEY,
                nickname TEXT NOT NULL,
                score INTEGER NOT NULL
            );",
        )
        .execute(&pool)
        .await
        .map_err(LeaderboardError::DatabaseSetup)?;

        Ok(Self { pool })
    }
}

pub async fn routes(auth_token: &'static str) -> Result<Router, LeaderboardError> {
    let state = LeaderboardConfig {
        form_submit_url: "http://localhost:1234/asdf",
        base_url: "http://localhost:3000",
        token: auth_token,
    };

    let database = Arc::new(Database::new().await?);

    Ok(Router::new()
        // the leaderboard
        .route("/", get(leaderboard))
        // submit from game
        .route("/backend/submit_score", post(submit_score))
        // frontend
        .route("/claim/list", get(unclaimed_scores_list))
        .route("/claim/{id}", get(claim_score_form))
        .route("/claim/{id}", post(claim_score_submit))
        // static stuff
        .route("/assets/style.css", get(style))
        .route("/assets/form.css", get(form_style))
        .route("/assets/font.ttf", get(font))
        .route("/assets/script.js", get(script))
        .route("/favicon.ico", get(icon))
        .route("/robots.txt", get(robots))
        // database + state
        .layer(Extension(database))
        .with_state(state))
}

async fn leaderboard(
    Extension(database): Extension<Arc<Database>>,
) -> Result<impl IntoResponse, LeaderboardError> {
    let scores =
        sqlx::query_as::<_, ScoreRow>("SELECT nickname, score FROM Scores ORDER BY score DESC;")
            .fetch_all(&database.pool)
            .await
            .map_err(LeaderboardError::FetchError)?;

    let mut placement_scores: Vec<PlacementScoreRow> = Vec::new();

    let mut last_score = -1;
    let mut last_placement = 1;
    for (i, score) in scores.into_iter().enumerate() {
        if last_score == score.score {
            placement_scores.push(PlacementScoreRow {
                nickname: score.nickname,
                score: score.score,
                placement: last_placement,
            });
        } else {
            last_score = score.score;
            last_placement = (i + 1).try_into().unwrap();
            placement_scores.push(PlacementScoreRow {
                nickname: score.nickname,
                score: score.score,
                placement: last_placement,
            });
        }
    }

    let leaderboard = LeaderboardTemplate {
        scores: placement_scores,
    }
    .render()
    .map_err(LeaderboardError::RenderError)?;

    Ok(Html(leaderboard))
}

async fn submit_score(
    headers: HeaderMap,
    State(state): State<LeaderboardConfig<'_>>,
    Extension(database): Extension<Arc<Database>>,
    Json(score): Json<RecievedScore>, // put every extractor above this!
) -> Result<impl IntoResponse, LeaderboardError> {
    let Some(authorization) = headers.get(AUTHORIZATION) else {
        return Err(LeaderboardError::MissingAuth);
    };

    if slow_equals(authorization.as_bytes(), state.token.as_bytes()) {
        return Err(LeaderboardError::WrongAuth);
    }

    if score.score < 0 {
        return Err(LeaderboardError::InvalidScore);
    }

    if score.color.len() != 7 {
        return Err(LeaderboardError::MalformedColor);
    }

    if !score.color.starts_with("#") {
        return Err(LeaderboardError::MalformedColor);
    }

    if score
        .color
        .chars()
        .filter(|x| x.is_ascii_hexdigit())
        .count()
        != 6
    {
        return Err(LeaderboardError::MalformedColor);
    }

    //todo!("Validate score::color");

    let id = Uuid::new_v4();

    // add score to unclaimed scores
    sqlx::query("INSERT INTO UnclaimedScores (id, score, color) VALUES (?, ?, ?);")
        .bind(id)
        .bind(score.score)
        .bind(score.color)
        .execute(&database.pool)
        .await
        .map_err(LeaderboardError::InsertFailure)?;

    Ok(Json(json!({"id": id.to_string()})))
}

async fn unclaimed_scores_list(
    Extension(database): Extension<Arc<Database>>,
) -> Result<impl IntoResponse, LeaderboardError> {
    let unclaimed_scores =
        sqlx::query_as::<_, UnclaimedScoreRow>("SELECT id, score, color FROM UnclaimedScores;")
            .fetch_all(&database.pool)
            .await
            .map_err(LeaderboardError::FetchError)?;

    let unclaimed = ClaimListTemplate { unclaimed_scores }
        .render()
        .map_err(LeaderboardError::RenderError)?;

    Ok(Html(unclaimed))
}

async fn claim_score_form(
    Path(id): Path<String>,
    Extension(database): Extension<Arc<Database>>,
) -> Result<impl IntoResponse, LeaderboardError> {
    let uuid = Uuid::from_str(&id).map_err(|_| LeaderboardError::InvalidId)?;

    let _unclaimed_scores = sqlx::query_as::<_, UnclaimedScoreRow>(
        "SELECT id, score, color FROM UnclaimedScores WHERE id = ?;",
    )
    .bind(uuid)
    .fetch_one(&database.pool)
    .await
    .map_err(LeaderboardError::FetchError)?;

    let form = ClaimFormTemplate {
        id: uuid,
        error_message: None,
    }
    .render()
    .map_err(LeaderboardError::RenderError)?;

    Ok(Html(form))
}

async fn claim_score_submit(
    State(state): State<LeaderboardConfig<'_>>,
    Path(id): Path<String>,
    Extension(database): Extension<Arc<Database>>,
    Form(claim): Form<ClaimScore>, // put every extractor above this!
) -> Result<impl IntoResponse, LeaderboardError> {
    let id = Uuid::from_str(&id).map_err(|_| LeaderboardError::InvalidId)?;

    let mut submit_form = false;
    let mut sanitized_nickname: Option<String> = None;

    let score = sqlx::query_as::<_, UnclaimedScoreRow>(
        "SELECT id, score, color FROM UnclaimedScores WHERE id = ?;",
    )
    .bind(id)
    .fetch_one(&database.pool)
    .await
    .map_err(LeaderboardError::FetchError)?;

    // leaderboard submission
    if let Some(wants_leaderboard) = claim.wants_leaderboard {
        if wants_leaderboard && claim.nickname.trim_end().is_empty() {
            //LeaderboardError::IncompleteData
            //return Ok(Redirect::to(&format!("{}/claim/{}", state.base_url, id)));
            todo!("Redirect back to form, nickname not provided");
        }

        sanitized_nickname = Some(helper::sanitize_name(claim.nickname.trim_end().to_string()));
    };

    if let Some(wants_raffle) = claim.wants_raffle {
        if wants_raffle
            && (claim.email.trim_end().is_empty()
                || claim.firstname.trim_end().is_empty()
                || claim.lastname.trim_end().is_empty())
        {
            todo!("Redirect back to form, something not provided");
        }

        submit_form = true;
    }

    // ----------- RACE CONDITION ?! -----------

    // delete score
    sqlx::query("DELETE FROM UnclaimedScores WHERE id = ?")
        .bind(id)
        .execute(&database.pool)
        .await
        .map_err(LeaderboardError::DeleteError)?;

    if let Some(nickname) = sanitized_nickname {
        sqlx::query("INSERT INTO Scores (nickname, score) VALUES (?, ?);")
            .bind(nickname)
            .bind(score.score)
            .execute(&database.pool)
            .await
            .map_err(LeaderboardError::InsertFailure)?;
    }

    if submit_form {
        let form_data: HPIFormSubmission = claim.into();

        let client = reqwest::Client::new();
        client
            .post(state.form_submit_url)
            .form(&form_data)
            .send()
            .await
            .map_err(LeaderboardError::TransmitError)?;
    }

    Ok(Redirect::to(&format!("{}/claim/list", state.base_url)))
}
