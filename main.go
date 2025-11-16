package main

import (
  "errors"
  "log/slog"
  "net/http"
  "os"
  "time"

  "github.com/brooknullsh/railway-playground/internal/handler"
  "github.com/brooknullsh/railway-playground/internal/store"
  "github.com/labstack/echo/v4"
  "github.com/lmittmann/tint"
)

func main() {
  opts := tint.Options{
    Level:      slog.LevelDebug,
    TimeFormat: time.Kitchen,
  }

  overwrite := tint.NewHandler(os.Stderr, &opts)
  logger := slog.New(overwrite)
  slog.SetDefault(logger)

  store, err := store.NewAndConnect()
  if err != nil {
    slog.Error("creating database pool", "error", err)
    os.Exit(1)
  }

  defer store.Pool.Close()

  var port string
  if port = os.Getenv("PORT"); port == "" {
    port = ":8080"
  } else {
    port = ":" + port
  }

  app := echo.New()
  handler.InitialiseWithState(app, store)

  if err := app.Start(port); err != nil && !errors.Is(err, http.ErrServerClosed) {
    slog.Error("server crash", "error", err)
  }
}
