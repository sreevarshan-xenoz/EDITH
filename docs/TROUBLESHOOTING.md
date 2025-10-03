# Troubleshooting Guide

## Quick Diagnostics

### Health Check Commands
```bash
# Test basic functionality
llm-wrapper "Hello, world!"

# Check backend connectivity
curl http://localhost:11434/api/tags

# Verify configuration
llm-wrapper enhanced stats

# Test performance
load_test --concurrency 1 --requests 5
```

### System Information
```bash
# Check Rust version
rustc --version

# Check binary version
llm-wrapper --version

# Check system resources
free -h
df -h
```

## Common Issues

### 1. Connection Issues

#### "Connection refused" or "Backend not found"

**Symptoms:**
- Error messages about connection failures
- "Backend 'ollama' not found" errors
- Timeouts when sending requests

**Diagnosis:**
```bash
# Check if Ollama is running
ps aux | grep ollama
curl http://localhost:11434/api/tags

# Check port availability
netstat -tlnp | grep 11434
```

**Solutions:**

1. **Start Ollama:**
   ```bash
   ollama serve
   ```

2. **Check configuration:**
   ```toml
   [backends.ollama]
   base_url = "http://localhost:11434"  # Verify correct URL
   timeout = "60s"                      # Increase timeout
   ```

3. **Test different port:**
   ```bash
   # If Ollama runs on different port
   ollama serve --port 11435
   ```
   
   Update config:
   ```toml
   [backends.ollama]
   base_url = "http://localhost:11435"
   ```

4. **Check firewall:**
   ```bash
   # Allow port through firewall
   sudo ufw allow 11434
   ```

#### "Request timeout" errors

**Symptoms:**
- Requests hang and eventually timeout
- Slow response times

**Solutions:**

1. **Increase timeout:**
   ```toml
   [backends.ollama]
   timeout = "120s"  # Increase from default 30s
   ```

2. **Check model size:**
   ```bash
   # Use smaller/faster model
   ollama pull llama3.2:7b  # Instead of larger variants
   ```

3. **Monitor system resources:**
   ```bash
   # Check CPU and memory usage
   htop
   
   # Check GPU usage (if applicable)
   nvidia-smi
   ```

### 2. Cache Issues

#### "Cache error" or poor cache performance

**Symptoms:**
- Cache-related error messages
- Low cache hit ratios (< 50%)
- High memory usage

**Diagnosis:**
```bash
# Check cache statistics
llm-wrapper enhanced cache stats

# Check cache directory
ls -la .cache/
du -sh .cache/

# Check disk space
df -h .
```

**Solutions:**

1. **Clear corrupted cache:**
   ```bash
   llm-wrapper enhanced cache clear
   rm -rf .cache/
   ```

2. **Adjust cache settings:**
   ```toml
   [cache]
   max_memory_entries = 500      # Reduce if memory constrained
   ttl = "30m"                   # Shorter TTL for dynamic content
   enable_persistence = false    # Disable if disk issues
   memory_pressure_threshold = 0.7  # Lower threshold
   ```

3. **Fix permissions:**
   ```bash
   chmod -R 755 .cache/
   chown -R $USER:$USER .cache/
   ```

4. **Monitor memory usage:**
   ```bash
   # Check memory consumption
   ps aux | grep llm-wrapper
   
   # Set memory limits
   ulimit -v 2097152  # 2GB virtual memory limit
   ```

#### Cache hit ratio too low

**Symptoms:**
- Cache hit ratio < 80%
- Slow response times despite caching

**Solutions:**

1. **Increase cache size:**
   ```toml
   [cache]
   max_memory_entries = 2000
   max_memory_bytes = 419430400  # 400MB
   ```

2. **Increase TTL:**
   ```toml
   [cache]
   ttl = "2h"  # Longer cache lifetime
   ```

3. **Check query patterns:**
   ```bash
   # Enable debug logging to see cache behavior
   RUST_LOG=debug llm-wrapper enhanced interactive
   ```

### 3. Template Issues

#### "Template not found" errors

**Symptoms:**
- Template-related error messages
- Templates not loading

**Diagnosis:**
```bash
# List available templates
llm-wrapper enhanced template list

# Check template directory
ls -la templates/

# Verify template syntax
llm-wrapper enhanced template show template_name
```

**Solutions:**

1. **Create template directory:**
   ```bash
   mkdir -p templates/
   ```

2. **Check template file:**
   ```bash
   # Verify template exists and is readable
   cat templates/your_template.hbs
   ```

3. **Fix template syntax:**
   ```handlebars
   <!-- Ensure proper Handlebars syntax -->
   Hello {{name}}!
   
   {{#if condition}}
   Content here
   {{/if}}
   ```

4. **Update configuration:**
   ```toml
   [templates]
   template_dir = "templates"  # Verify correct path
   auto_reload = true
   ```

#### Template rendering errors

**Symptoms:**
- Template syntax errors
- Variables not substituted
- Slow template rendering

**Solutions:**

1. **Validate template syntax:**
   ```bash
   # Test template with simple variables
   echo '{"name": "test"}' | llm-wrapper enhanced chat-template your_template --vars -
   ```

2. **Check variable names:**
   ```handlebars
   <!-- Ensure variable names match exactly -->
   Hello {{user_name}}!  <!-- Not {{username}} -->
   ```

3. **Simplify complex templates:**
   ```handlebars
   <!-- Avoid deeply nested conditions -->
   {{#if simple_condition}}
   Simple content
   {{/if}}
   ```

### 4. Performance Issues

#### Slow response times

**Symptoms:**
- First token time > 200ms
- Overall slow performance
- High CPU/memory usage

**Diagnosis:**
```bash
# Check performance metrics
llm-wrapper enhanced stats

# Run performance test
load_test --concurrency 1 --requests 10 --output perf.json

# Monitor system resources
top -p $(pgrep llm-wrapper)
```

**Solutions:**

1. **Optimize model selection:**
   ```bash
   # Use faster models
   ollama pull llama3.2:7b      # Instead of 13b or 70b
   ollama pull phi3:mini        # Very fast small model
   ```

2. **Tune cache settings:**
   ```toml
   [cache]
   max_memory_entries = 5000    # Larger cache
   ttl = "4h"                   # Longer TTL
   ```

3. **Optimize backend settings:**
   ```toml
   [backends.ollama]
   timeout = "30s"              # Reasonable timeout
   
   [backends.ollama.rate_limit]
   max_concurrent = 10          # Higher concurrency
   ```

4. **System optimization:**
   ```bash
   # Increase file descriptor limits
   ulimit -n 4096
   
   # Use faster storage for cache
   # Mount tmpfs for cache directory
   sudo mount -t tmpfs -o size=1G tmpfs .cache/
   ```

#### High memory usage

**Symptoms:**
- Memory usage continuously growing
- Out of memory errors
- System becoming unresponsive

**Solutions:**

1. **Reduce cache size:**
   ```toml
   [cache]
   max_memory_entries = 100
   max_memory_bytes = 52428800  # 50MB
   memory_pressure_threshold = 0.6
   ```

2. **Enable memory monitoring:**
   ```bash
   # Monitor memory usage
   watch -n 1 'ps aux | grep llm-wrapper'
   ```

3. **Restart periodically:**
   ```bash
   # Add to cron for long-running processes
   0 */6 * * * pkill llm-wrapper && sleep 5 && llm-wrapper enhanced interactive
   ```

### 5. UI Issues

#### Terminal UI not displaying correctly

**Symptoms:**
- Garbled text in terminal
- Layout issues
- Colors not working

**Solutions:**

1. **Check terminal compatibility:**
   ```bash
   # Verify terminal supports required features
   echo $TERM
   tput colors
   ```

2. **Update terminal:**
   ```bash
   # Use modern terminal emulator
   # Recommended: Alacritty, iTerm2, Windows Terminal
   ```

3. **Enable high contrast mode:**
   ```bash
   # Press F6 in interactive mode
   # Or set in config:
   ```
   ```toml
   [ui]
   high_contrast = true
   ```

4. **Adjust terminal size:**
   ```bash
   # Ensure minimum terminal size
   resize -s 24 80  # 24 rows, 80 columns minimum
   ```

#### Keyboard shortcuts not working

**Symptoms:**
- Function keys not responding
- Ctrl combinations not working

**Solutions:**

1. **Check terminal key mapping:**
   ```bash
   # Test key detection
   showkey -a
   ```

2. **Use alternative shortcuts:**
   - Instead of F1-F4: Use commands in chat
   - Instead of Ctrl+Q: Use `/quit` command
   - Instead of Ctrl+L: Use `/clear` command

3. **Terminal-specific fixes:**
   ```bash
   # For tmux users
   set -g xterm-keys on
   
   # For screen users
   termcapinfo xterm* ti@:te@
   ```

### 6. Configuration Issues

#### Configuration not loading

**Symptoms:**
- Default settings always used
- Configuration changes ignored
- "Configuration error" messages

**Diagnosis:**
```bash
# Check config file location
ls -la enhanced-config.toml

# Validate TOML syntax
toml-test enhanced-config.toml  # If toml-test installed
```

**Solutions:**

1. **Fix TOML syntax:**
   ```toml
   # Ensure proper TOML format
   [cache]
   max_memory_entries = 1000  # Number, not string
   ttl = "1h"                 # String with quotes
   enable_persistence = true  # Boolean, not string
   ```

2. **Check file permissions:**
   ```bash
   chmod 644 enhanced-config.toml
   ```

3. **Validate configuration:**
   ```bash
   # Test configuration loading
   llm-wrapper enhanced stats  # Should show config values
   ```

#### Invalid configuration values

**Symptoms:**
- "Validation error" messages
- Unexpected behavior

**Solutions:**

1. **Check value ranges:**
   ```toml
   [cache]
   memory_pressure_threshold = 0.8  # Must be 0.1-1.0
   max_memory_entries = 100         # Must be > 0
   
   [streaming]
   max_concurrent_streams = 5       # Must be > 0
   buffer_size = 8192              # Must be >= 1024
   ```

2. **Verify URLs:**
   ```toml
   [backends.ollama]
   base_url = "http://localhost:11434"  # Valid URL format
   ```

3. **Check file paths:**
   ```toml
   [templates]
   template_dir = "templates"  # Directory must exist
   
   [cache]
   cache_dir = ".cache"       # Must be writable
   ```

## Advanced Troubleshooting

### Debug Logging

Enable detailed logging for diagnosis:

```bash
# Set log level
export RUST_LOG=debug

# Or trace level for maximum detail
export RUST_LOG=trace

# Run with logging
llm-wrapper enhanced interactive 2>&1 | tee debug.log
```

### Performance Profiling

```bash
# Run load test with detailed output
load_test --concurrency 5 --requests 50 --output detailed_metrics.json

# Analyze results
cat detailed_metrics.json | jq '.performance_metrics'

# Check for bottlenecks
grep "duration_ms" detailed_metrics.json | sort -n
```

### Memory Profiling

```bash
# Use valgrind for memory analysis (Linux)
valgrind --tool=memcheck --leak-check=full llm-wrapper enhanced interactive

# Use heaptrack (Linux)
heaptrack llm-wrapper enhanced interactive

# Monitor with system tools
while true; do
    ps aux | grep llm-wrapper | grep -v grep
    sleep 5
done
```

### Network Debugging

```bash
# Monitor network traffic
sudo tcpdump -i lo port 11434

# Check connection details
ss -tulpn | grep 11434

# Test with curl
curl -v http://localhost:11434/api/tags
```

## Recovery Procedures

### Complete Reset

If all else fails, perform a complete reset:

```bash
# 1. Stop all processes
pkill llm-wrapper
pkill ollama

# 2. Clear all data
rm -rf .cache/
rm -rf templates/
rm enhanced-config.toml

# 3. Restart services
ollama serve &
sleep 5

# 4. Test basic functionality
llm-wrapper "Hello, world!"

# 5. Reconfigure as needed
llm-wrapper enhanced interactive
```

### Backup and Restore

```bash
# Backup configuration and templates
tar -czf llm-wrapper-backup.tar.gz enhanced-config.toml templates/ .cache/

# Restore from backup
tar -xzf llm-wrapper-backup.tar.gz
```

## Getting Help

### Information to Collect

When reporting issues, include:

1. **System information:**
   ```bash
   uname -a
   rustc --version
   llm-wrapper --version
   ```

2. **Configuration:**
   ```bash
   cat enhanced-config.toml
   ```

3. **Error messages:**
   ```bash
   # Full error output
   llm-wrapper enhanced stats 2>&1
   ```

4. **Performance metrics:**
   ```bash
   llm-wrapper enhanced stats --export debug_metrics.json
   ```

5. **System resources:**
   ```bash
   free -h
   df -h
   ps aux | grep -E "(llm-wrapper|ollama)"
   ```

### Support Channels

- **Documentation**: Check README.md and docs/ directory
- **GitHub Issues**: Search existing issues before creating new ones
- **Community Forums**: Check community discussions
- **Debug Mode**: Use `RUST_LOG=debug` for detailed logs

### Creating Bug Reports

Include in bug reports:
- Clear description of the issue
- Steps to reproduce
- Expected vs actual behavior
- System information (above)
- Configuration files
- Error messages and logs
- Performance metrics if relevant

This troubleshooting guide should help resolve most common issues. For persistent problems, don't hesitate to seek community support with detailed information about your setup and the specific issue you're experiencing.