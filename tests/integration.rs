use sigmaos_svntogit_community::*;

#[test]
fn component_is_constructible_and_runnable() {
    // Arrange
    let c = sigmaos_svntogit_community::SvntogitCommunity::new("sigmaos-svntogit-community").expect("valid name");
    // Act
    let out = c.run().expect("run succeeds");
    // Assert
    assert!(out.contains("sigmaos-svntogit-community"));
}
