package handler

import (
  "log/slog"
  "net/http"
  "os"

  "github.com/brooknullsh/railway-playground/internal/store"
  "github.com/gofiber/fiber/v3"
)

const SECRET_KEY = "secret"

type Handler struct {
  store *store.Store
}

func (this *Handler) Index(ctx fiber.Ctx) error {
  secret := fiber.MustGetState[string](ctx.App().State(), SECRET_KEY)
  slog.Info("leaking secret", "secret", secret)

  if status := this.store.SetRefreshToken(ctx, "token", 1); status != http.StatusOK {
    return ctx.SendStatus(status)
  }

  return ctx.SendStatus(http.StatusOK)
}

func Setup(app *fiber.App, store *store.Store) {
  var handler Handler
  handler.store = store

  secret := os.Getenv("SECRET")
  if secret == "" {
    slog.Error("unset $SECRET variable")
    os.Exit(1)
  }

  app.State().Set(SECRET_KEY, secret)
  app.Get("/", handler.Index)
}
