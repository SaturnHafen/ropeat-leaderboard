use std::str::FromStr;

use axum_test::TestServer;
use backend::routes;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const GOOD_TOKEN: &str = "asdf";
const BAD_TOKEN: &str = "asdf1";

const GOOD_SCORE: i32 = 1337;
const GOOD_SCORE_2: i32 = 13337;
const GOOD_COLOR: &str = "#123456";
const GOOD_COLOR_2: &str = "#987654";

const NORMAL_NICKNAME: &str = "HELLO_TESTING!";
const NORMAL_NICKNAME_2: &str = "BYE_TESTING!";

const BAD_NICKNAME: &str = "<script>alert(\"'&1'\");</script>";
const SANITIZED_NICKNAME: &str =
    "&lt;script&gt;alert(&quot;&#39;&amp;1&#39;&quot;);&lt;/script&gt;";

#[derive(Serialize)]
struct GoodScoreFormat {
    score: i32,
    color: String,
}

#[derive(Serialize)]
struct BadScoreFormat {}

#[derive(Serialize)]
struct GoodFormSubmitFormat {
    wants_leaderboard: Option<bool>,
    wants_raffle: Option<bool>,

    nickname: String,

    email: String,
    firstname: String,
    lastname: String,
}

#[derive(Serialize)]
struct BadFormSubmitFormat {}

#[derive(Deserialize)]
struct SubmitResponse {
    id: String,
}

async fn setup_server() -> TestServer {
    TestServer::new(routes(GOOD_TOKEN).await.unwrap()).unwrap()
}

async fn submit_score(server: &TestServer) -> Uuid {
    let payload = GoodScoreFormat {
        score: GOOD_SCORE,
        color: GOOD_COLOR.to_string(),
    };

    let response = server
        .post("/backend/submit_score")
        .authorization(GOOD_TOKEN)
        .json(&payload)
        .await;

    response.assert_status_ok();
    let json: SubmitResponse = response.json();

    Uuid::from_str(&json.id).unwrap()
}

async fn submit_score2(server: &TestServer) -> Uuid {
    let payload = GoodScoreFormat {
        score: GOOD_SCORE_2,
        color: GOOD_COLOR_2.to_string(),
    };

    let response = server
        .post("/backend/submit_score")
        .authorization(GOOD_TOKEN)
        .json(&payload)
        .await;

    response.assert_status_ok();
    let json: SubmitResponse = response.json();

    Uuid::from_str(&json.id).unwrap()
}

async fn claim_score(server: &TestServer) {
    claim_score_username(server, NORMAL_NICKNAME).await
}

async fn claim_score_2(server: &TestServer) {
    let id = submit_score2(server).await;

    let body = GoodFormSubmitFormat {
        wants_leaderboard: Some(true),
        wants_raffle: None,

        nickname: NORMAL_NICKNAME_2.to_string(),
        email: "".to_string(),
        firstname: "".to_string(),
        lastname: "".to_string(),
    };

    let response = server.post(&format!("/claim/{}", id)).form(&body).await;

    //response.assert_status_ok();
}

async fn claim_score_username(server: &TestServer, nickname: &str) {
    let id = submit_score(server).await;

    let body = GoodFormSubmitFormat {
        wants_leaderboard: Some(true),
        wants_raffle: None,

        nickname: nickname.to_string(),
        email: "".to_string(),
        firstname: "".to_string(),
        lastname: "".to_owned(),
    };

    let response = server.post(&format!("/claim/{}", id)).form(&body).await;

    //response.assert_status_ok();
}

#[tokio::test] // happy path
async fn no_submitted_score_shows_expected_text() {
    let server = setup_server().await;

    let response = server.get("/claim/list").await;

    response.assert_status_ok();
    response.assert_text_contains(
        "Aktuell gibt es keine Scores, die noch keinen Nutzernamen zugeordnet wurden. Hast du das Spiel bereits verlassen?",
    );
}

#[tokio::test] // happy path
async fn submitted_but_unclaimed_score_shows_on_list() {
    let server = setup_server().await;

    let payload = GoodScoreFormat {
        score: GOOD_SCORE,
        color: GOOD_COLOR.to_string(),
    };

    let response = server
        .post("/backend/submit_score")
        .authorization(GOOD_TOKEN)
        .json(&payload)
        .await;

    response.assert_status_ok();

    let response = server.get("/claim/list").await;

    response.assert_status_ok();
    response.assert_text_contains(GOOD_SCORE.to_string());
}

#[tokio::test]
async fn visiting_right_score_id_returns_claim_form() {
    // when visiting the right uuid, we get a claim form
    let server = setup_server().await;

    let id = submit_score(&server).await;

    let response = server.get(&format!("/claim/{}", id)).await;
    response.assert_status_ok();
}

#[tokio::test] // happy path
async fn claimed_score_shows_up_on_leaderboard() {
    // after claiming a score, the score shows up on the leaderboard
    let server = setup_server().await;

    let id = submit_score(&server).await;

    let body = GoodFormSubmitFormat {
        wants_leaderboard: Some(true),
        wants_raffle: None,

        nickname: NORMAL_NICKNAME.to_string(),
        email: "".to_string(),
        firstname: "".to_string(),
        lastname: "".to_owned(),
    };

    let response = server.post(&format!("/claim/{}", id)).form(&body).await;

    response.assert_status(StatusCode::SEE_OTHER); // we want a redirect to /claim/list

    let response = server.get("/").await;

    response.assert_status_ok();
    response.assert_text_contains(NORMAL_NICKNAME);
    response.assert_text_contains(GOOD_SCORE.to_string());
}

#[tokio::test]
async fn different_scores_are_ordered_correctly() {
    let server = setup_server().await;

    claim_score(&server).await;
    claim_score_2(&server).await;

    let response = server.get("/").await;
    response.assert_text_contains(NORMAL_NICKNAME);
    response.assert_text_contains(NORMAL_NICKNAME_2);
    response.assert_text_contains(GOOD_SCORE.to_string());
    response.assert_text_contains(GOOD_SCORE_2.to_string());

    let text = response.text();
    let pos1 = text.find(NORMAL_NICKNAME).unwrap();
    let pos2 = text.find(NORMAL_NICKNAME_2).unwrap();

    assert!(pos1 > pos2, "Higher score should be first");

    let pos1 = text.find(&GOOD_SCORE.to_string()).unwrap();
    let pos2 = text.find(&GOOD_SCORE_2.to_string()).unwrap();
    assert!(pos1 > pos2, "Higher score should be first");
}

#[tokio::test] // happy path
async fn same_scores_get_same_position() {
    let server = setup_server().await;

    claim_score(&server).await;
    claim_score(&server).await;
    claim_score_2(&server).await;

    let response = server.get("/").await;
    response.assert_text_contains(NORMAL_NICKNAME);
    response.assert_text_contains(NORMAL_NICKNAME_2);
    response.assert_text_contains(GOOD_SCORE.to_string());
    response.assert_text_contains(GOOD_SCORE_2.to_string());

    let re = regex::Regex::new(&format!(
        "1.*{NORMAL_NICKNAME_2}.*{GOOD_SCORE_2}.*2.*{NORMAL_NICKNAME}.*{GOOD_SCORE}.*2.*{NORMAL_NICKNAME}.*{GOOD_SCORE}"
    ))
    .unwrap();

    assert!(re.is_match(&response.text().replace("\n", "")))
}

#[tokio::test]
async fn cant_submit_score_without_token() {
    let server = setup_server().await;

    let payload = GoodScoreFormat {
        score: GOOD_SCORE,
        color: GOOD_COLOR.to_string(),
    };

    let response = server.post("/backend/submit_score").json(&payload).await;

    response.assert_status_not_ok();
}

#[tokio::test]
async fn cant_submit_score_with_wrong_token() {
    let server = setup_server().await;

    let payload = GoodScoreFormat {
        score: GOOD_SCORE,
        color: GOOD_COLOR.to_string(),
    };

    let response = server
        .post("/backend/submit_score")
        .authorization(BAD_TOKEN)
        .json(&payload)
        .await;

    response.assert_status_not_ok();
}

#[tokio::test]
async fn bad_submitted_score_doesnt_show_on_unclaimed_list() {
    // when uploading a score from the game with an invalid token, this score is not present in the unclaimed scores list
    let server = setup_server().await;

    let payload = GoodScoreFormat {
        score: GOOD_SCORE,
        color: GOOD_COLOR.to_string(),
    };

    let response = server
        .post("/backend/submit_score")
        .authorization(BAD_TOKEN)
        .json(&payload)
        .await;

    response.assert_status_not_ok();

    let response = server.get("/claim/list").await;

    response.assert_status_ok();
    assert!(
        !response.text().contains(&GOOD_SCORE.to_string()),
        "The score shows up!"
    );
}

#[tokio::test]
#[ignore = "not implemented"]
async fn double_claim_score_doesnt_work() {
    // when submitting a claim form twice for the same score, the second one doesn't work
    todo!()
}

#[tokio::test]
async fn username_gets_filtered_for_html_chars() {
    // when submitting a claim form with html chars in the username, these chars get html encoded
    let server = setup_server().await;

    claim_score_username(&server, BAD_NICKNAME).await;

    let response = server.get("/").await;

    response.assert_status_ok();
    response.assert_text_contains(GOOD_SCORE.to_string());
    let text = response.text();

    assert!(!text.contains(BAD_NICKNAME));
    response.assert_text_contains(SANITIZED_NICKNAME);
}

#[tokio::test]
#[ignore = "not implemented"]
async fn form_submit_unset_checkboxes_dont_copy_internally() {
    // when submitting a claim form with text in the boxes, these don't get transfered to the form submissions / leaderboard
    let server = setup_server().await;

    let id = submit_score(&server).await;

    let body = GoodFormSubmitFormat {
        wants_leaderboard: Some(true),
        wants_raffle: None,

        nickname: NORMAL_NICKNAME.to_string(),
        email: "".to_string(),
        firstname: "".to_string(),
        lastname: "".to_owned(),
    };

    let response = server.post(&format!("/claim/{}", id)).form(&body).await;

    todo!()
}

#[tokio::test]
#[ignore = "not implemented"]
async fn website_form_submit_works_always() {
    // if you submit the website form, it always works without returning an error

    todo!()
}

#[tokio::test]
#[ignore = "not implemented"]
async fn cant_load_page_with_unknown_id() {
    // If you try to load a page with an unknown (but valid) id, it fails and redirects to the list page

    todo!()
}
