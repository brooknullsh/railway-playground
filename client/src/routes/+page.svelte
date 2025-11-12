<script lang="ts">
  // import type { PageProps } from "./$types"

  // let { data }: PageProps = $props()
</script>

<p>Hello, world!</p>

<!-- {#await data.users}
  Loading...
{:then users}
  {#each users as user}
    <div class="flex h-6 items-center">
      <p class="w-44">{user.lastName}, {user.firstName}</p>
      <p class="w-12">{user.age}</p>
      <p class="w-44">{user.mobile}</p>
      {#if user.isPro}
        <div class="size-2 bg-green-400 rounded-full"></div>
      {:else}
        <div class="size-2 bg-red-400 rounded-full"></div>
      {/if}
    </div>
  {/each}
{:catch error}
  <p class="text-red-400">Unable to retrieve users: {error.message}</p>
{/await} -->
