use futures::stream::{self, StreamExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("GITLAB_TOKEN")?;
    let project_ids: Vec<u64> = vec![138, 125, 156, 889, 594];
    let client = reqwest::Client::new();

    let bodies = stream::iter(project_ids)
        .map(|project_id| {
            let client = &client;
            let token = &token;
            async move {
                client
                    .get(format!(
                        "https://gitlab.ppro.com/api/v4/projects/{}/jobs",
                        project_id
                    ))
                    .header("PRIVATE-TOKEN", token)
                    .send()
                    .await?
                    .text()
                    .await
            }
        })
        .buffer_unordered(5);

    bodies
        .for_each(|body| async {
            match body {
                Ok(b) => println!("Ok: {}", b.len()),
                Err(e) => eprintln!("Error: {}", e),
            }
        })
        .await;

    Ok(())
}
