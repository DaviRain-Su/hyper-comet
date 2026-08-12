//! Project grouping for Launch Studio (Phase 2.5).

use comet_proto::StudioLaunch;

/// Group launches by `project_name`, falling back to `"Ungrouped"`.
/// Groups are sorted by name; launches inside a group keep caller order.
pub fn group_launches(launches: &[StudioLaunch]) -> Vec<(String, Vec<&StudioLaunch>)> {
    let mut groups: Vec<(String, Vec<&StudioLaunch>)> = Vec::new();
    for launch in launches {
        let name = launch
            .project_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("Ungrouped")
            .to_string();
        if let Some((_, bucket)) = groups.iter_mut().find(|(existing, _)| existing == &name) {
            bucket.push(launch);
        } else {
            groups.push((name, vec![launch]));
        }
    }
    groups.sort_by(|a, b| a.0.cmp(&b.0));
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    fn launch(id: &str, project: Option<&str>) -> StudioLaunch {
        let mut launch = StudioLaunch::new_now();
        launch.id = id.into();
        launch.title = id.into();
        launch.project_name = project.map(str::to_string);
        launch
    }

    #[test]
    fn groups_by_project_name_and_keeps_ungrouped() {
        let launches = vec![
            launch("a", Some("RWA")),
            launch("b", None),
            launch("c", Some("RWA")),
            launch("d", Some("Escrow")),
        ];
        let groups = group_launches(&launches);
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].0, "Escrow");
        assert_eq!(groups[1].0, "RWA");
        assert_eq!(groups[1].1.len(), 2);
        assert_eq!(groups[2].0, "Ungrouped");
    }
}
