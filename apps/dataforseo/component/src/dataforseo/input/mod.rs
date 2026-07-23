//! Closed typed model-visible input DTOs.

mod ai;
mod backlinks;
mod content_domain;
mod page_business;
mod search;

pub(super) use ai::{
    AiKeywordVolumeInput, DomainScope, KeywordScope, LlmMentionsInput, LlmPlatform, MatchType,
    SearchFilter, TargetEntity,
};
pub(super) use backlinks::{
    BacklinksInput, BacklinksStatus, BacklinksSummaryInput, ReferringDomainsInput,
};
pub(super) use content_domain::{
    ContentSearchInput, ContentSentimentInput, DomainTechnologiesInput, DomainWhoisInput, PageType,
};
pub(super) use page_business::{BusinessInfoInput, BusinessSearchInput, InstantPageAuditInput};
pub(super) use search::{
    GoogleSerpInput, KeywordOverviewInput, KeywordSuggestionsInput, RankedKeywordsInput,
};
