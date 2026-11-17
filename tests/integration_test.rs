//! Integration tests for SigmaOS-svntogit-community.

use sigmaos_svntogit_community_lib::Component;

#[test]
fn component_initialises_and_runs() {
    let mut c = Component::new();
    c.init().expect("init should succeed");
    c.run().expect("run should succeed after init");
}
