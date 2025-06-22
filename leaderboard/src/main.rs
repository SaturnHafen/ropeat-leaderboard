use backend::{routes, LeaderboardError};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), LeaderboardError> {
    let auth_token = "abcd".to_string();
    let app: axum::Router = routes(auth_token).await?;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .map_err(|x| -> LeaderboardError { LeaderboardError::TcpListener(x) })?;

    println!("Running on http://localhost:3000/");

    axum::serve(listener, app)
        .await
        .map_err(|x| -> LeaderboardError { LeaderboardError::AxumServer(x) })?;

    Ok(())
}
