# AgentNOC Web UI

React-based web interface for AgentNOC.

## Setup

Install dependencies:

```bash
npm install
```

## Development

Run the development server:

```bash
npm run dev
```

The dev server will proxy API requests to `http://localhost:7654`.

## Build

Build for production:

```bash
npm run build
```

This creates the production build in the `dist/` directory, which the Rust server serves.

## Integration with Rust Server

The Rust binary automatically serves the built React app from `web-ui/dist/` and opens it in the browser on startup.

Make sure to build the React app before running the Rust binary:

```bash
cd web-ui
npm install
npm run build
cd ..
cargo run
```


