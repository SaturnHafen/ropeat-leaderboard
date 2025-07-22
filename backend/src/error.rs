use axum::{
    body::Body,
    response::{IntoResponse, Response},
};
use reqwest::StatusCode;
use serde_json::json;

use crate::submission::SubmissionError;

#[derive(Debug)]
pub enum LeaderboardError {
    AxumServer(std::io::Error),
    TcpListener(std::io::Error),
    DatabaseSetup(sqlx::Error),
    TransactionBeginError(sqlx::Error),
    MissingAuth,
    WrongAuth,
    InvalidId,
    TransmitError(SubmissionError),
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
                write!(fmt, "Couldn't start the server! Reason: {x}")
            }
            LeaderboardError::TcpListener(x) => {
                write!(fmt, "Couldn't launch TcpListener! Reason: {x}")
            }
            LeaderboardError::DatabaseSetup(x) => {
                write!(
                    fmt,
                    "Something went wrong while starting the database! Reason: {x}"
                )
            }
            LeaderboardError::TransactionBeginError(x) => {
                write!(fmt, "Couldn't start transaction. Reason: {x}")
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
                write!(fmt, "Couldn't transmit data to the HPI server! Reason: {x}")
            }
            LeaderboardError::IncompleteData(x) => {
                write!(fmt, "You didn't provide all necessary data points! ({x})")
            }
            LeaderboardError::InsertFailure(x) => {
                write!(fmt, "Something went wrong while inserting! Reason: {x}")
            }
            LeaderboardError::FetchError(x) => {
                write!(fmt, "Couldn't fetch leaderboard! Reason: {x}")
            }
            LeaderboardError::DeleteError(x) => {
                write!(fmt, "Couldn't delete unclaimed score! Reason: {x}")
            }
            LeaderboardError::RenderError(x) => {
                write!(fmt, "Couldn't render template! Reason: {x}")
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
            LeaderboardError::TransmitError(x) => {
                if cfg!(debug_assertions) {
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from(format!("TransmitError: {x}")))
                        .unwrap()
                } else {
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from("Wir konnten dich leider nicht in das Gewinnspiel-Formular eintragen. Bitte frage einen der anwesenden Standbetreuenden um Hilfe!"))
                        .unwrap()
                }
            }
            LeaderboardError::InsertFailure(_) => todo!("implement `500 internal server error`"),
            LeaderboardError::FetchError(x) => {
                if cfg!(debug_assertions) {
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from(format!("FetchError: {x}")))
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

impl From<SubmissionError> for LeaderboardError {
    fn from(value: SubmissionError) -> Self {
        LeaderboardError::TransmitError(value)
    }
}
