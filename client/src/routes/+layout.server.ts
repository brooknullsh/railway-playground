import { env } from "$env/dynamic/private"
import type { LayoutServerLoad } from "./$types"

type User = {
  id: number
  age: number
  isPro: boolean
  mobile: string
  lastName: string
  firstName: string
}

async function fetchAllUsers() {
  const response = await fetch(`${env.SERVER_URL}/`)

  console.log(await response.text())
  // return response.json() as Promise<User[]>
}

async function fetchUser() {
  const response = await fetch(`${env.SERVER_URL}/auth/login`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ firstName: "Alice" }),
  })

  console.log(response.status)
  // return response.json() as Promise<User>
}

export const load: LayoutServerLoad = async () => {
  return { users: fetchUser() }
}
