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
  return response.json() as Promise<User[]>
}

export const load: LayoutServerLoad = async () => {
  return { users: fetchAllUsers() }
}
