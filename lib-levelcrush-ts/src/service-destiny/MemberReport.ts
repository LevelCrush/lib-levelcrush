// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.
import type { MemberReportActivity } from "./MemberReportActivity";
import type { MemberReportActivityMode } from "./MemberReportActivityMode";
import type { MemberReportFireteamMember } from "./MemberReportFireteamMember";
import type { MemberReportStats } from "./MemberReportStats";
import type { MemberResponse } from "./MemberResponse";
import type { MemberTitle } from "./MemberTitle";

export interface MemberReport { version: number, membership_id: number, display_name_global: string, last_played_at: number, activity_timestamps: Record<number,number>, activity_attempts: number, activity_attempts_with_clan: number, activity_completions: number, stats_pve: MemberReportStats, stats_pvp: MemberReportStats, stats_gambit: MemberReportStats, stats_private_matches: MemberReportStats, stats_reckoning: MemberReportStats, top_activity_modes: Array<MemberReportActivityMode>, top_activities: Array<MemberReportActivity>, activity_map: Record<string, MemberReportActivity>, frequent_clan_members: Array<MemberReportFireteamMember>, frequent_non_clan_members: Array<MemberReportFireteamMember>, total_clan_members: number, total_non_clan_members: number, titles: Array<MemberTitle>, member: MemberResponse, }