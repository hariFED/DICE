"use client"

import { useQuery } from "@tanstack/react-query"
import { fetchNodes, fetchRounds, fetchQueue, fetchStats } from "./api"
import { REFRESH_INTERVAL } from "./constants"

export function useNodes() {
  return useQuery({
    queryKey: ["nodes"],
    queryFn: fetchNodes,
    refetchInterval: REFRESH_INTERVAL,
  })
}

export function useRounds() {
  return useQuery({
    queryKey: ["rounds"],
    queryFn: fetchRounds,
    refetchInterval: 3000,
  })
}

export function useQueue() {
  return useQuery({
    queryKey: ["queue"],
    queryFn: fetchQueue,
    refetchInterval: REFRESH_INTERVAL,
  })
}

export function useStats() {
  return useQuery({
    queryKey: ["stats"],
    queryFn: fetchStats,
    refetchInterval: REFRESH_INTERVAL,
  })
}
