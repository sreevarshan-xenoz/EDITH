#!/usr/bin/env rust-script

//! Security audit script for Enhanced LLM Wrapper
//! 
//! This script performs various security checks:
//! - Template injection vulnerabilities
//! - Input validation
//! - File system access controls
//! - Memory safety checks
//! - Configuration security

use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("🔒 Enhanced LLM Wrapper Security Audit");
    println!("═══════════════════════════════════════");
    
    let mut issues = Vec::new();
    let mut warnings = Vec::new();
    
    // Check 1: Template Security
    check_template_security(&mut issues, &mut warnings);
    
    // Check 2: Configuration Security
    check_configuration_security(&mut issues, &mut warnings);
    
    // Check 3: File System Security
    check_filesystem_security(&mut issues, &mut warnings);
    
    // Check 4: Dependency Security
    check_dependency_security(&mut issues, &mut warnings);
    
    // Check 5: Input Validation
    check_input_validation(&mut issues, &mut warnings);
    
    // Check 6: Memory Safety
    check_memory_safety(&mut issues, &mut warnings);
    
    // Report Results
    print_security_report(&issues, &warnings);
    
    // Exit with appropriate code
    if !issues.is_empty() {
        std::process::exit(1);
    } else if !warnings.is_empty() {
        std::process::exit(2);
    } else {
        std::process::exit(0);
    }
}

fn check_template_security(issues: &mut Vec<String>, warnings: &mut Vec<String>) {
    println!("\n📝 Checking Template Security...");
    
    // Check for template sandboxing
    if let Ok(content) = fs::read_to_string("src/template.rs") {
        if !content.contains("enable_sandboxing") {
            issues.push("Template sandboxing not implemented".to_string());
        } else {
            println!("✅ Template sandboxing implemented");
        }
        
        if !content.contains("max_template_size") {
            warnings.push("Template size limits not enforced".to_string());
        } else {
            println!("✅ Template size limits implemented");
        }
        
        if !content.contains("max_render_time") {
            warnings.push("Template render time limits not enforced".to_string());
        } else {
            println!("✅ Template render time limits implemented");
        }
    } else {
        issues.push("Cannot read template.rs for security analysis".to_string());
    }
    
    // Check template directory permissions
    if Path::new("templates").exists() {
        if let Ok(metadata) = fs::metadata("templates") {
            let permissions = metadata.permissions();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = permissions.mode();
                if mode & 0o002 != 0 {
                    issues.push("Template directory is world-writable".to_string());
                } else {
                    println!("✅ Template directory permissions secure");
                }
            }
        }
    }
}

fn check_configuration_security(issues: &mut Vec<String>, warnings: &mut Vec<String>) {
    println!("\n⚙️ Checking Configuration Security...");
    
    // Check for configuration validation
    if let Ok(content) = fs::read_to_string("src/config.rs") {
        if !content.contains("validate") {
            issues.push("Configuration validation not implemented".to_string());
        } else {
            println!("✅ Configuration validation implemented");
        }
        
        if content.contains("password") && !content.contains("redact") {
            warnings.push("Potential password logging without redaction".to_string());
        }
    }
    
    // Check configuration file permissions
    if Path::new("enhanced-config.toml").exists() {
        if let Ok(metadata) = fs::metadata("enhanced-config.toml") {
            let permissions = metadata.permissions();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = permissions.mode();
                if mode & 0o044 != 0 {
                    warnings.push("Configuration file is readable by others".to_string());
                } else {
                    println!("✅ Configuration file permissions secure");
                }
            }
        }
    }
}

fn check_filesystem_security(issues: &mut Vec<String>, warnings: &mut Vec<String>) {
    println!("\n📁 Checking File System Security...");
    
    // Check cache directory security
    if let Ok(content) = fs::read_to_string("src/cache.rs") {
        if content.contains("../") && !content.contains("canonicalize") {
            issues.push("Potential path traversal vulnerability in cache".to_string());
        }
        
        if !content.contains("create_dir_all") {
            warnings.push("Cache directory creation may not be secure".to_string());
        } else {
            println!("✅ Cache directory handling implemented");
        }
    }
    
    // Check for unsafe file operations
    let source_files = ["src/lib.rs", "src/cache.rs", "src/template.rs", "src/config.rs"];
    for file in &source_files {
        if let Ok(content) = fs::read_to_string(file) {
            if content.contains("std::fs::remove_dir_all") && !content.contains("canonicalize") {
                warnings.push(format!("Potentially unsafe directory removal in {}", file));
            }
            
            if content.contains("File::create") && !content.contains("permissions") {
                warnings.push(format!("File creation without explicit permissions in {}", file));
            }
        }
    }
}

fn check_dependency_security(issues: &mut Vec<String>, warnings: &mut Vec<String>) {
    println!("\n📦 Checking Dependency Security...");
    
    // Run cargo audit if available
    let audit_output = Command::new("cargo")
        .args(&["audit", "--format", "json"])
        .output();
    
    match audit_output {
        Ok(output) if output.status.success() => {
            let audit_result = String::from_utf8_lossy(&output.stdout);
            if audit_result.contains("\"vulnerabilities\"") {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&audit_result) {
                    if let Some(vulns) = json.get("vulnerabilities").and_then(|v| v.as_array()) {
                        if !vulns.is_empty() {
                            issues.push(format!("Found {} security vulnerabilities in dependencies", vulns.len()));
                        } else {
                            println!("✅ No known vulnerabilities in dependencies");
                        }
                    }
                }
            } else {
                println!("✅ No known vulnerabilities in dependencies");
            }
        }
        Ok(_) => {
            warnings.push("cargo audit found issues (run 'cargo audit' for details)".to_string());
        }
        Err(_) => {
            warnings.push("cargo audit not available (install with 'cargo install cargo-audit')".to_string());
        }
    }
    
    // Check Cargo.toml for security-sensitive dependencies
    if let Ok(content) = fs::read_to_string("Cargo.toml") {
        let security_sensitive = ["openssl", "rustls", "ring", "webpki"];
        for dep in &security_sensitive {
            if content.contains(dep) {
                println!("ℹ️ Using security-sensitive dependency: {}", dep);
            }
        }
    }
}

fn check_input_validation(issues: &mut Vec<String>, warnings: &mut Vec<String>) {
    println!("\n🔍 Checking Input Validation...");
    
    let source_files = ["src/lib.rs", "src/main.rs", "src/template.rs", "src/ui.rs"];
    
    for file in &source_files {
        if let Ok(content) = fs::read_to_string(file) {
            // Check for potential injection points
            if content.contains("format!") && content.contains("user") {
                warnings.push(format!("Potential format string injection in {}", file));
            }
            
            // Check for SQL-like operations (though we don't use SQL)
            if content.contains("query") && !content.contains("validate") {
                warnings.push(format!("Query operations without validation in {}", file));
            }
            
            // Check for command execution
            if content.contains("Command::new") && !content.contains("sanitize") {
                warnings.push(format!("Command execution without input sanitization in {}", file));
            }
            
            // Check for deserialization
            if content.contains("from_str") && content.contains("serde") {
                if !content.contains("validate") {
                    warnings.push(format!("Deserialization without validation in {}", file));
                }
            }
        }
    }
    
    println!("✅ Input validation checks completed");
}

fn check_memory_safety(issues: &mut Vec<String>, warnings: &mut Vec<String>) {
    println!("\n🧠 Checking Memory Safety...");
    
    // Check for unsafe blocks
    let source_files = ["src/lib.rs", "src/cache.rs", "src/template.rs", "src/ui.rs", "src/streaming.rs"];
    let mut unsafe_count = 0;
    
    for file in &source_files {
        if let Ok(content) = fs::read_to_string(file) {
            let unsafe_blocks = content.matches("unsafe").count();
            unsafe_count += unsafe_blocks;
            
            if unsafe_blocks > 0 {
                warnings.push(format!("Found {} unsafe blocks in {}", unsafe_blocks, file));
            }
        }
    }
    
    if unsafe_count == 0 {
        println!("✅ No unsafe blocks found");
    } else {
        println!("⚠️ Found {} total unsafe blocks", unsafe_count);
    }
    
    // Check for potential memory leaks
    for file in &source_files {
        if let Ok(content) = fs::read_to_string(file) {
            if content.contains("Box::leak") {
                issues.push(format!("Potential memory leak with Box::leak in {}", file));
            }
            
            if content.contains("forget") {
                warnings.push(format!("Memory forget operation in {}", file));
            }
        }
    }
}

fn print_security_report(issues: &[String], warnings: &[String]) {
    println!("\n📊 Security Audit Report");
    println!("═══════════════════════════");
    
    if issues.is_empty() && warnings.is_empty() {
        println!("✅ No security issues found!");
        println!("🎉 The Enhanced LLM Wrapper passes all security checks.");
    } else {
        if !issues.is_empty() {
            println!("\n🚨 CRITICAL ISSUES ({}):", issues.len());
            for (i, issue) in issues.iter().enumerate() {
                println!("  {}. {}", i + 1, issue);
            }
        }
        
        if !warnings.is_empty() {
            println!("\n⚠️ WARNINGS ({}):", warnings.len());
            for (i, warning) in warnings.iter().enumerate() {
                println!("  {}. {}", i + 1, warning);
            }
        }
        
        println!("\n📋 Recommendations:");
        if !issues.is_empty() {
            println!("  • Address all critical issues before production deployment");
        }
        if !warnings.is_empty() {
            println!("  • Review and address warnings as appropriate");
        }
        println!("  • Run regular security audits");
        println!("  • Keep dependencies updated");
        println!("  • Monitor for new security advisories");
    }
    
    println!("\n🔒 Security Status:");
    if issues.is_empty() {
        if warnings.is_empty() {
            println!("  Status: ✅ SECURE");
        } else {
            println!("  Status: ⚠️ SECURE WITH WARNINGS");
        }
    } else {
        println!("  Status: 🚨 NEEDS ATTENTION");
    }
}