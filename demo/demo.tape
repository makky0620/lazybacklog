Output demo.gif

Set Width 220
Set Height 50
Set FontSize 14
Set Theme "Dracula"
Set Framerate 24

Type "../target/release/lazybacklog --demo"
Enter
Sleep 1s

# Select first project
Enter
Sleep 500ms

# Browse issues with j (vi-style)
Type j
Sleep 300ms
Type j
Sleep 300ms
Type j
Sleep 300ms

# Open issue detail
Enter
Sleep 500ms

# Close detail
Escape
Sleep 500ms

# Start search (sleep after / to let UI enter search mode)
Type /
Sleep 300ms

# Type query — matches "Fix login page layout on mobile" (DEMO-2)
Type "login"
Sleep 500ms

# Show match highlight
Sleep 1s

# Exit search mode before quitting
Escape
Sleep 300ms

# Quit
Type q
