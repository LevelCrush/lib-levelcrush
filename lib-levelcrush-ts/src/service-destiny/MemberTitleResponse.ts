// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.
import type { MemberClanInformation } from "./MemberClanInformation";
import type { MemberTitle } from "./MemberTitle";

export interface MemberTitleResponse { display_name: string, display_name_platform: string, membership_id: string, membership_platform: bigint, timestamp_last_played: number, raid_report: string, clan?: MemberClanInformation, titles: Array<MemberTitle>, }