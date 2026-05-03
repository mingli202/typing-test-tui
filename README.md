# Typing test tui

Monkeytype inspired typing test in the terminal. Can play offline using the built-in quotes or from the backend. Can also play multiplayer with others online or make your own lobby for your friends

## Stack
### Tui client
- Rust
- Has built-in quotes. Can also make a request to the Go backend for more sources (from monkeytype)
- Ratatui, crossterm, tokio

### Backend
- Go
- Handles multiplayer via websockets
- Join lobbies (DONE) or match randomly (TODO)
- go std lib, gorilla/websockets
