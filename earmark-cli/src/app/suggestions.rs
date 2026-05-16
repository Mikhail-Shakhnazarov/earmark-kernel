pub(crate) fn next_commands_for_run(run_id: &str) -> Vec<String> {
    vec![
        format!("em run explain {}", run_id),
        format!("em run timeline {}", run_id),
        format!("em run artifacts {}", run_id),
        format!("em run graph {}", run_id),
        format!("em failure list --run-id {}", run_id),
    ]
}

pub(crate) fn next_commands_for_assignment(
    assignment: &earmark_core::TransitionAssignment,
) -> Vec<String> {
    let mut commands = vec![format!("em run explain {}", assignment.run_id)];
    if let Some(hid) = &assignment.handoff_manifest_id {
        commands.push(format!("em handoff explain {}", hid.as_str()));
    }
    if let Some(did) = &assignment.completion_change_set_id {
        commands.push(format!("em change-set explain {}", did.as_str()));
    }
    commands.push(format!("em run timeline {}", assignment.run_id));
    commands
}

pub(crate) fn next_commands_for_change_set(change_set: &earmark_core::ChangeSet) -> Vec<String> {
    let mut commands = vec![format!("em run explain {}", change_set.run_id)];
    if let Some(hid) = &change_set.handoff_manifest_id {
        commands.push(format!("em handoff explain {}", hid.as_str()));
    }
    if let Some(aid) = &change_set.assignment_id {
        commands.push(format!("em assignment explain {}", aid.as_str()));
    }
    commands.push(format!("em run timeline {}", change_set.run_id));
    commands
}

pub(crate) fn next_commands_for_handoff(handoff: &earmark_core::HandoffManifest) -> Vec<String> {
    let mut commands = vec![format!("em run explain {}", handoff.run_id)];
    if let Some(transition_id) = &handoff.to_transition_id {
        commands.push(format!(
            "em workflow run <workflow_id> --system-id <system_id> --handoff {} # successor {}",
            handoff.id.as_str(),
            transition_id
        ));
    } else {
        commands.push(format!(
            "em workflow run <workflow_id> --system-id <system_id> --handoff {}",
            handoff.id.as_str()
        ));
    }
    commands.push(format!("em run timeline {}", handoff.run_id));
    commands
}

pub(crate) fn next_commands_for_failure(
    failure_id: &str,
    failure: &earmark_core::TransformationFailure,
) -> Vec<String> {
    let mut commands = vec![
        format!("em failure show {}", failure_id),
        format!("em run explain {}", failure.run_id),
    ];
    if let Some(delta_id) = &failure.failed_change_set_id {
        commands.push(format!("em change-set explain {}", delta_id.as_str()));
    }
    commands.push(format!(
        "em assignment explain {}",
        failure.assignment_id.as_str()
    ));
    commands.push(format!("em run timeline {}", failure.run_id));
    commands
}
