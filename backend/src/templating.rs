use askama::Template;
use uuid::Uuid;

use crate::{PlacementScoreRow, UnclaimedScoreRow};

#[derive(Template)]
#[template(path = "index.html", escape = "none")]
pub struct LeaderboardTemplate {
    pub scores: Vec<PlacementScoreRow>,
}

#[derive(Template)]
#[template(path = "claim_list.html")]
pub struct ClaimListTemplate {
    pub unclaimed_scores: Vec<UnclaimedScoreRow>,
}

#[derive(Template)]
#[template(path = "claim_form.html")]
pub struct ClaimFormTemplate {
    pub id: Uuid,
    pub error_message: Option<String>,
}
