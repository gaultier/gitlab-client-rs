use futures::stream::{self, StreamExt};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Pipeline {
    id: u64,
    project_id: u64,
    #[serde(rename(deserialize = "ref"))]
    reference: Option<String>,
    sha: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Job {
    created_at: Option<String>,
    started_at: Option<String>,
    finished_at: Option<String>,
    duration: Option<f64>,
    id: u64,
    name: Option<String>,
    pipeline: Pipeline,
}

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
                Ok((project_id, b)) => {
                    println!("[{}] Ok: {}", project_id, b.len());
                    let jobs: Vec<Job> = serde_json::from_str(&b).unwrap();
                    println!("[{}] Ok: {:#?}", project_id, jobs);
                }
                Err((project_id, e)) => eprintln!("[{}] Error: {}", project_id, e),
            }
        })
        .await;

    Ok(())
}
