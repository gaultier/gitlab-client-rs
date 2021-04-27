use futures::stream::{self, StreamExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("GITLAB_TOKEN")?;
    let project_ids: Vec<u64> = vec![138, 125, 156, 889, 594];
    let client = reqwest::Client::new();
    let projects_count = project_ids.len();

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
                    .await
                    .map_err(|err| (project_id, err))?
                    .text()
                    .await
                    .map(|body| (project_id, body))
                    .map_err(|err| (project_id, err))
            }
        })
        .buffer_unordered(projects_count);

    bodies
        .for_each(|body| async {
            match body {
                Ok((project_id, b)) => println!("[{}] Ok: {}", project_id, b.len()),
                Err((project_id, e)) => eprintln!("[{}] Error: {}", project_id, e),
            }
        })
        .await;

    Ok(())
}
