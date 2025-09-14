use ashpd::desktop::file_chooser::{FileFilter, OpenFileRequest};

pub async fn launch_file_picker(
    save: bool,
    title: String,
    file_name: String,
    filters: Vec<String>,
) -> String {
    let mut file_filter = FileFilter::new(filters.join(", ").as_str());

    for filter in filters {
        file_filter = file_filter.glob(&filter);
    }

    if !save {
        let request = OpenFileRequest::default()
            .directory(false)
            // .identifier(identifier)
            .modal(true)
            .title(&*title)
            .multiple(false)
            .filters(vec![file_filter]);
        // .current_filter(current_filter)

        match request.send().await.and_then(|r| r.response()) {
            Ok(files) => files
                .uris()
                .first()
                .map(|u| u.to_string())
                .unwrap_or_default(),
            Err(err) => {
                eprintln!("Failed to open a file: {err}");
                "".to_string()
            }
        }
    } else {
        use ashpd::desktop::file_chooser::SaveFileRequest;

        let request = SaveFileRequest::default()
            // .identifier(identifier)
            .modal(true)
            .title(&*title)
            .current_name(&*file_name)
            .filters(vec![file_filter]);
        // .current_filter(current_filter)

        match request.send().await.and_then(|r| r.response()) {
            Ok(files) => files
                .uris()
                .first()
                .map(|u| u.to_string())
                .unwrap_or_default(),
            Err(err) => {
                eprintln!("Failed to open a file: {err}");
                "".to_string()
            }
        }
    }
}
