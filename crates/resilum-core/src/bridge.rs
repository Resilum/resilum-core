//! Per-spec bridge tasks. The runtime is not implemented yet; each spec becomes
//! a placeholder task that holds its slot under supervision.

use crate::spec::Specs;
use crate::supervisor::Task;

/// One supervised task per configured spec (bridges, VPN, covert).
pub(crate) fn tasks_for(specs: &Specs) -> Vec<Task> {
    let mut tasks = Vec::new();
    for b in &specs.bridges {
        tasks.push(stub(format!("bridge/{}", b.services.join("+"))));
    }
    for v in &specs.vpn {
        tasks.push(stub(format!("vpn/{:?}", v.mode)));
    }
    for c in &specs.covert {
        tasks.push(stub(format!("covert/{}", c.carrier)));
    }
    tasks
}

fn stub(label: String) -> Task {
    Task::new(label.clone(), move || run_stub(label.clone()))
}

async fn run_stub(label: String) {
    tracing::info!(bridge = %label, "running (stub — runtime not implemented)");
    std::future::pending::<()>().await
}

#[cfg(test)]
mod tests {
    use super::tasks_for;
    use crate::spec;

    #[test]
    fn one_task_per_spec() {
        let yaml = "\
bridges:
  - mode: listen
    service: tor
    identity: x
    tcp: 127.0.0.1:9050
covert:
  - carrier: icmp
";
        let specs = spec::load(yaml).unwrap();
        assert_eq!(tasks_for(&specs).len(), 2);
    }
}
