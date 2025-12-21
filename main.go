package main

import (
  "log/slog"
  "os"
  "strings"

  "github.com/brooknullsh/railway-playground/internal/handler"
  "github.com/brooknullsh/railway-playground/internal/store"
  "github.com/gofiber/fiber/v3"
)

func init() {
  var opts slog.HandlerOptions

  lvl := os.Getenv("LOG_LEVEL")
  switch strings.ToLower(lvl) {
  case "debug":
    opts.Level = slog.LevelDebug
  case "info":
    opts.Level = slog.LevelInfo
  case "warn":
    opts.Level = slog.LevelWarn
  case "error":
    opts.Level = slog.LevelError
  default:
    opts.Level = slog.LevelDebug
  }

  overwrite := slog.NewTextHandler(os.Stdout, &opts)
  logger := slog.New(overwrite)
  slog.SetDefault(logger)
}

func main() {
  var store store.Store
  if err := store.MutInit(); err != nil {
    slog.Error("initialising a new store", "error", err)
    os.Exit(1)
  }

  app := fiber.New()
  handler.Setup(app, &store)

  port := ":"
  if port += os.Getenv("PORT"); port == ":" {
    port += "8080"
  }

  if err := app.Listen(port); err != nil {
    slog.Error("server crash", "error", err)

    // NOTE: Clean-up here instead of deferred above. "The program terminates
    // immediately; deferred functions are not run" - os.Exit
    store.Close()
    os.Exit(1)
  }
}
