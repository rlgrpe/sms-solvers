//! Example demonstrating configuration presets and validation.
//!
//! This example shows how to use the built-in configuration presets
//! and how to validate custom configurations.
//!
//! # Running
//!
//! ```bash
//! cargo run --example config_presets
//! ```

use sms_solvers::SmsSolverServiceConfig;
use std::time::Duration;

fn main() {
    println!("=== SMS Solvers Configuration Presets ===\n");

    // Fast preset - for development/testing
    let fast = SmsSolverServiceConfig::fast();
    println!("Fast preset:");
    println!("  Timeout: {:?}", fast.timeout);
    println!("  Poll interval: {:?}", fast.poll_interval);
    println!("  Use case: Development and testing\n");

    // Balanced preset (default) - for most production use cases
    let balanced = SmsSolverServiceConfig::balanced();
    println!("Balanced preset (default):");
    println!("  Timeout: {:?}", balanced.timeout);
    println!("  Poll interval: {:?}", balanced.poll_interval);
    println!("  Use case: Most production environments\n");

    // Patient preset - for slow providers or unreliable networks
    let patient = SmsSolverServiceConfig::patient();
    println!("Patient preset:");
    println!("  Timeout: {:?}", patient.timeout);
    println!("  Poll interval: {:?}", patient.poll_interval);
    println!("  Use case: Slow providers or unreliable networks\n");

    // Custom configuration with builder
    println!("=== Custom Configuration with Builder ===\n");

    let custom = SmsSolverServiceConfig::builder()
        .timeout(Duration::from_secs(90))
        .poll_interval(Duration::from_secs(2))
        .build();
    println!("Custom config:");
    println!("  Timeout: {:?}", custom.timeout);
    println!("  Poll interval: {:?}", custom.poll_interval);

    // Validate the config
    match custom.validate() {
        Ok(()) => println!("  Validation: PASSED\n"),
        Err(e) => println!("  Validation: FAILED - {}\n", e),
    }

    // Using try_build for validated construction
    println!("=== Validated Configuration with try_build ===\n");

    // Valid config
    match SmsSolverServiceConfig::builder()
        .timeout(Duration::from_secs(60))
        .poll_interval(Duration::from_secs(2))
        .try_build()
    {
        Ok(config) => {
            println!("Valid config created:");
            println!("  Timeout: {:?}", config.timeout);
            println!("  Poll interval: {:?}\n", config.poll_interval);
        }
        Err(e) => println!("Failed to create config: {}\n", e),
    }

    // Invalid: timeout too short
    println!("Testing validation - timeout too short:");
    match SmsSolverServiceConfig::builder()
        .timeout(Duration::from_secs(5)) // Below minimum of 10s
        .try_build()
    {
        Ok(_) => println!("  Unexpectedly succeeded\n"),
        Err(e) => println!("  Expected error: {}\n", e),
    }

    // Invalid: poll interval exceeds timeout
    println!("Testing validation - poll interval exceeds timeout:");
    match SmsSolverServiceConfig::builder()
        .timeout(Duration::from_secs(30))
        .poll_interval(Duration::from_secs(60))
        .try_build()
    {
        Ok(_) => println!("  Unexpectedly succeeded\n"),
        Err(e) => println!("  Expected error: {}\n", e),
    }

    // Invalid: poll interval too short
    println!("Testing validation - poll interval too short:");
    match SmsSolverServiceConfig::builder()
        .poll_interval(Duration::from_millis(50)) // Below minimum of 100ms
        .try_build()
    {
        Ok(_) => println!("  Unexpectedly succeeded\n"),
        Err(e) => println!("  Expected error: {}\n", e),
    }

    // Chaining with_* methods on presets
    println!("=== Customizing Presets ===\n");

    let custom_fast = SmsSolverServiceConfig::fast()
        .with_timeout(Duration::from_secs(45))
        .with_poll_interval(Duration::from_millis(500));

    println!("Fast preset with custom values:");
    println!("  Timeout: {:?}", custom_fast.timeout);
    println!("  Poll interval: {:?}", custom_fast.poll_interval);

    match custom_fast.validate() {
        Ok(()) => println!("  Validation: PASSED"),
        Err(e) => println!("  Validation: FAILED - {}", e),
    }
}
