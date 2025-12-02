package main

import (
  "log/slog"
  "os"
  "time"

  "github.com/brooknullsh/railway-playground/internal/handler"
  "github.com/brooknullsh/railway-playground/internal/store"
  "github.com/gofiber/fiber/v3"
  "github.com/lmittmann/tint"
)

func init() {
  opts := tint.Options{
    Level:      slog.LevelDebug,
    TimeFormat: time.Kitchen,
  }

  overwrite := tint.NewHandler(os.Stderr, &opts)
  logger := slog.New(overwrite)
  slog.SetDefault(logger)
}

func main() {
  store, err := store.NewAndConnect()
  if err != nil {
    slog.Error("creating the store", "error", err)
    os.Exit(1)
  }

  app := fiber.New()
  handler.InitialiseWithState(app, store)

  var port string
  if port = os.Getenv("PORT"); port == "" {
    port = ":8080"
  } else {
    port = ":" + port
  }

  if err := app.Listen(port); err != nil {
    slog.Error("starting server", "error", err)
    os.Exit(1)
  }
}
