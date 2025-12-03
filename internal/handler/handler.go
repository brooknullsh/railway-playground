package handler

import (
  "net/http"

  "github.com/brooknullsh/railway-playground/internal/store"
  "github.com/gofiber/fiber/v3"
)

func Root(ctx fiber.Ctx) error {
  store, cast := fiber.GetState[*store.Store](ctx.App().State(), "store")
  if !cast {
    return ctx.SendStatus(http.StatusInternalServerError)
  }

  user, code := store.GetUserByName(ctx, "Alice")
  if code != http.StatusOK {
    return ctx.SendStatus(code)
  }

  return ctx.SendString(user.FirstName)
}
