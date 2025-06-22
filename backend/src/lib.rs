mod r#static;
mod templating;

use askama::Template;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{
        header::{self, AUTHORIZATION},
        HeaderMap, Response,
    },
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
    Extension, Form, Json, Router,
};
use r#static::{font, icon, robots, style};
use reqwest::StatusCode;
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

use crate::r#static::{form_style, script};

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
    wants_key: Option<bool>,
    wants_hpi: Option<bool>,

    email: String,
    email_hpi: String,
    nickname: String,
    name: String,
}

#[derive(Debug)]
pub enum LeaderboardError {
    AxumServer(std::io::Error),
    TcpListener(std::io::Error),
    DatabaseSetup(sqlx::Error),
    TransactionBeginError(sqlx::Error),
    MissingAuth,
    WrongAuth,
    InvalidId,
    TransmitError(reqwest::Error),
    InsertFailure(sqlx::Error),
    FetchError(sqlx::Error),
    DeleteError(sqlx::Error),
    RenderError(askama::Error),
    InvalidScore,
    MalformedColor,
    IncompleteData(String),
}

impl std::fmt::Display for LeaderboardError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LeaderboardError::AxumServer(x) => {
                write!(fmt, "Couldn't start the server! Reason: {}", x)
            }
            LeaderboardError::TcpListener(x) => {
                write!(fmt, "Couldn't launch TcpListener! Reason: {}", x)
            }
            LeaderboardError::DatabaseSetup(x) => {
                write!(
                    fmt,
                    "Something went wrong while starting the database! Reason: {}",
                    x
                )
            }
            LeaderboardError::TransactionBeginError(x) => {
                write!(fmt, "Couldn't start transaction. Reason: {}", x)
            }
            LeaderboardError::MissingAuth => {
                write!(fmt, "You didn't provide any credentials!")
            }
            LeaderboardError::WrongAuth => {
                write!(fmt, "You didn't provide valid credentials!")
            }
            LeaderboardError::InvalidId => {
                write!(fmt, "You didn't provide a valid id!")
            }
            LeaderboardError::TransmitError(x) => {
                write!(
                    fmt,
                    "Couldn't transmit data to the HPI server! Reason: {}",
                    x
                )
            }
            LeaderboardError::IncompleteData(x) => {
                write!(fmt, "You didn't provide all necessary data points! ({})", x)
            }
            LeaderboardError::InsertFailure(x) => {
                write!(fmt, "Something went wrong while inserting! Reason: {}", x)
            }
            LeaderboardError::FetchError(x) => {
                write!(fmt, "Couldn't fetch leaderboard! Reason: {}", x)
            }
            LeaderboardError::DeleteError(x) => {
                write!(fmt, "Couldn't delete unclaimed score! Reason: {}", x)
            }
            LeaderboardError::RenderError(x) => {
                write!(fmt, "Couldn't render template! Reason: {}", x)
            }
            LeaderboardError::InvalidScore => {
                write!(fmt, "The score is not valid!")
            }
            LeaderboardError::MalformedColor => {
                write!(fmt, "The color is not valid!")
            }
        }
    }
}

impl std::error::Error for LeaderboardError {}

impl IntoResponse for LeaderboardError {
    fn into_response(self) -> axum::response::Response {
        match self {
            LeaderboardError::AxumServer(_)
            | LeaderboardError::TcpListener(_)
            | LeaderboardError::DatabaseSetup(_) => {
                unreachable!("The server is not even up!")
            }
            LeaderboardError::MissingAuth => Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(Body::from("You didn't provide a authorization token!"))
                .unwrap(),
            LeaderboardError::WrongAuth => Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(Body::from(
                    "You didn't provide a valid authorization token!",
                ))
                .unwrap(),
            LeaderboardError::InvalidId => Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(
                    "The given id is malformed! Where did you get it from?",
                ))
                .unwrap(),
            LeaderboardError::IncompleteData(x) => Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from("you didn't enter all necessary data pieces"))
                .unwrap(),
            LeaderboardError::TransactionBeginError(_) => {
                todo!("implement `500 internal server error`")
            }
            LeaderboardError::TransmitError(_) => todo!("implement `500 internal server error`"),
            LeaderboardError::InsertFailure(_) => todo!("implement `500 internal server error`"),
            LeaderboardError::FetchError(x) => {
                if cfg!(debug_assertions) {
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from(format!("FetchError: {}", x)))
                        .unwrap()
                } else {
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::empty())
                        .unwrap()
                }
            }
            LeaderboardError::DeleteError(_) => todo!("implement error"),
            LeaderboardError::RenderError(_) => todo!("implement `500 internal server error`"),
            LeaderboardError::InvalidScore => Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"error": "Invalid score"}).to_string()))
                .unwrap(),
            LeaderboardError::MalformedColor => Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"error": "Malformed color"}).to_string()))
                .unwrap(),
        }
    }
}

#[derive(Clone)]
struct LeaderboardState {
    base_url: String,
    token: String,
}

#[derive(Debug, Clone, Serialize)]
struct HPIFormSubmission {
    name: String,
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

pub async fn routes(auth_token: String) -> Result<Router, LeaderboardError> {
    let state = LeaderboardState {
        base_url: "http://localhost:3000".to_string(),
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
    State(state): State<LeaderboardState>,
    Extension(database): Extension<Arc<Database>>,
    Json(score): Json<RecievedScore>, // put every extractor above this!
) -> Result<impl IntoResponse, LeaderboardError> {
    let Some(authorization) = headers.get(AUTHORIZATION) else {
        return Err(LeaderboardError::MissingAuth);
    };

    if authorization.as_bytes() != state.token.as_bytes() {
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

async fn claim_score_form(Path(id): Path<String>) -> Result<impl IntoResponse, LeaderboardError> {
    let form = ClaimFormTemplate {
        id: Uuid::from_str(&id).map_err(|_| LeaderboardError::InvalidId)?,
        error_message: None,
    }
    .render()
    .map_err(LeaderboardError::RenderError)?;

    Ok(Html(form))
}

async fn claim_score_submit(
    State(state): State<LeaderboardState>,
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

        sanitized_nickname = Some(sanitize_name(claim.nickname.trim_end().to_string()));
    };

    if let Some(wants_key) = claim.wants_key {
        if wants_key && claim.email.trim_end().is_empty() {
            todo!("Redirect back to form, email not provided");
        }
    };

    if let Some(wants_hpi) = claim.wants_hpi {
        if wants_hpi && (claim.email.trim_end().is_empty() || claim.name.trim_end().is_empty()) {
            todo!("Redirect back to form, email or name not provided");
        }

        submit_form = true;
    };

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
        let client = reqwest::Client::new();
        client
            .post("https://localhost:1337/asdf")
            .form(&HPIFormSubmission {
                name: "test".to_string(),
                email: "asdf@1245".to_string(),
            })
            .send()
            .await
            .map_err(LeaderboardError::TransmitError)?;
    }

    Ok(Redirect::to(&format!("{}/claim/list", state.base_url)))
}

fn sanitize_name(name: String) -> String {
    // See <https://stackoverflow.com/questions/7381974/which-characters-need-to-be-escaped-in-html#7382028>
    name.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&#39;")
}

#[test]
fn simple_xss_gets_replaced() {
    assert_eq!(
        sanitize_name("<script>alert(1);</script>".to_string()),
        "&lt;script&gt;alert(1);&lt;/script&gt;".to_string()
    );
}

#[test]
fn all_evil_chars_get_replaced() {
    assert_eq!(
        sanitize_name("&<>\"'".to_string()),
        "&amp;&lt;&gt;&quot;&#39;".to_string()
    )
}
