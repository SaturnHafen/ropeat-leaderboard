use reqwest::Response;
use serde::Serialize;

use regex::Regex;

use crate::LeaderboardError;

const HPI_FORM: &str = "https://hpi.de/registrierung/2025/gewinnspiel-gamescom-2025/";

#[derive(Debug)]
pub enum SubmissionError {
    TokenFetchFailed(reqwest::Error),
    TokenExtractFailed,
    SubmitFailed(reqwest::Error),
}

impl std::fmt::Display for SubmissionError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TokenFetchFailed(x) => {
                write!(
                    fmt,
                    "Couldn't fetch access token from HPI Website. Are we blocked? Internal Error: {x}"
                )
            }
            Self::TokenExtractFailed => {
                write!(
                    fmt,
                    "Couldn't extract token from HPI Website. Did the layout change?"
                )
            }
            Self::SubmitFailed(x) => {
                write!(fmt, "Couldn't submit data to HPI Website. Are we blocked, was the format changed or did the user enter something malformed? Internal Error: {x}")
            }
        }
    }
}

impl std::error::Error for SubmissionError {}

#[derive(Debug, Clone, Serialize)]
pub struct HPIFormData {
    #[serde(rename(serialize = "persons[0][first_name]"))]
    pub firstname: String,
    #[serde(rename(serialize = "persons[0][last_name]"))]
    pub lastname: String,
    #[serde(rename(serialize = "contactdetails_5[0][identification]"))]
    pub email: String,
    #[serde(rename(serialize = "registrationvarchars_103[0][registrationvarchar]"))]
    pub occupation: String,
    #[serde(rename(serialize = "registrationvarchars_105[0][registrationvarchar]"))]
    pub email_consent: String,
    #[serde(rename(serialize = "registrationvarchars_106[0][registrationvarchar]"))]
    pub data_processing_consent: String,
}

#[derive(Debug, Clone, Serialize)]
struct HPIFormDataFinalized {
    #[serde(flatten)]
    form_data: HPIFormData,
    #[serde(rename(serialize = "zz_id"))]
    id: String,
    #[serde(rename(serialize = "zz_action"))]
    action: &'static str, // "insert"
    #[serde(rename(serialize = "events_contacts[0][event_id]"))]
    event_id: u64, // 4062
}

fn create_filled_form(form_data: HPIFormData, id: String) -> HPIFormDataFinalized {
    HPIFormDataFinalized {
        form_data,
        id,
        action: "insert",
        event_id: 4062,
    }
}

async fn send_form(form: HPIFormDataFinalized) -> Result<Response, SubmissionError> {
    // "post" it
    let client = reqwest::Client::new();
    let response = client
        .post(HPI_FORM)
        .form(&form)
        .send()
        .await
        .map_err(|x| SubmissionError::SubmitFailed(x))?;

    Ok(response)
}

async fn get_form_id() -> Result<String, SubmissionError> {
    let re = Regex::new(r#"<input type="hidden" name="zz_id" value="(.{5,10})">"#).unwrap(); // This should never fail!

    // "get" html page
    let client = reqwest::Client::new();
    let response = client
        .get(HPI_FORM)
        .send()
        .await
        .map_err(|x| SubmissionError::TokenFetchFailed(x))?;

    // "parse" html page
    let response = response
        .text()
        .await
        .map_err(|x| SubmissionError::TokenFetchFailed(x))?;

    // extract token from match
    let result = re
        .captures(&response)
        .ok_or(SubmissionError::TokenExtractFailed)?;
    let token = result
        .get(1)
        .ok_or(SubmissionError::TokenExtractFailed)?
        .as_str();

    Ok(token.to_string())
}

pub async fn submit_form(form: HPIFormData) -> Result<(), SubmissionError> {
    let id = get_form_id().await?;
    let filled_form = create_filled_form(form, id);

    let response = send_form(filled_form).await?;

    println!("{:?}", response);
    println!("{}", response.text().await.unwrap());

    Ok(())
}

#[cfg(test)]
#[tokio::test]
#[ignore = "Makes requests to the HPI website"]
async fn form_id_works() {
    assert!(get_form_id().await.is_ok())
}

#[cfg(test)]
#[tokio::test]
#[ignore = "Makes requests to the HPI website"]
async fn easy_submission_works() {
    assert!(submit_form(HPIFormData {
        firstname: "Testy".to_string(),
        lastname: "McTestface".to_string(),
        email: "testy@example.com".to_string(),
        occupation: "Schüler:in".to_string(),
        email_consent: "yes".to_string(),
        data_processing_consent: "Ja, ich stimme zu.".to_string(),
    })
    .await
    .is_ok())
}

#[cfg(test)]
#[tokio::test]
#[ignore = "Makes requests to the HPI website"]
async fn submission_steps_work() {
    let token = get_form_id().await;
    assert!(token.is_ok());

    let token = token.unwrap();

    // create form submission
    let form = create_filled_form(
        HPIFormData {
            firstname: "Testy".to_string(),
            lastname: "McTestface".to_string(),
            email: "testy@example.com".to_string(),
            occupation: "Schüler:in".to_string(),
            email_consent: "yes".to_string(),
            data_processing_consent: "Ja, ich stimme zu.".to_string(),
        },
        token,
    );

    let response = send_form(form).await;
    println!("{:?}", response);
    assert!(response.is_ok());
}
