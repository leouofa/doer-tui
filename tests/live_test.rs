use doer_tui::api::ApiClient;
use doer_tui::checklist::checklist_progress;
use doer_tui::config::Config;

#[test]
fn test_live_instance() {
    let test_url = std::env::var("DOER_TEST_URL")
        .or_else(|_| std::env::var("DOER_REMOTE_URL"))
        .unwrap_or_else(|_| "http://localhost:3000".to_string());

    let config = Config::new(&test_url);
    let client = ApiClient::new(&config);

    if let Ok(stats) = client.get_stats() {
        assert!(stats.total_todo_lists >= 1);
        if let Ok(todo_lists) = client.list_todo_lists(None, None, None, None) {
            if let Some(first) = todo_lists.first() {
                if let Ok(detail) = client.get_todo_list(first.id) {
                    let (done, total) =
                        checklist_progress(detail.body.as_deref().unwrap_or(""));
                    println!(
                        "Checked Todo List {}: {} of {} tasks completed",
                        detail.title, done, total
                    );
                }
            }
        }
    } else {
        println!(
            "Live server at {} not reachable during test run (set DOER_TEST_URL to override)",
            test_url
        );
    }
}
