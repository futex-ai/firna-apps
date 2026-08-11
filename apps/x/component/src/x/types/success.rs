//! Model-visible success variants for every X tool.

use serde::Serialize;

use crate::x::metrics_types::GetPostMetricsOutput;
use crate::x::types::accounts::{ManageRelationshipOutput, UsersOutput};
use crate::x::types::discovery::{
    CommunitiesOutput, ListsOutput, MediaOutput, SpacesOutput, TrendsOutput,
};
use crate::x::types::list_actions::ManageListOutput;
use crate::x::types::media_actions::ManageMediaOutput;
use crate::x::types::messaging::{CreateBookmarkFolderOutput, DmsOutput, ManageDmOutput};
use crate::x::types::post_actions::ManagePostOutput;
use crate::x::types::posts::{
    CreatePostOutput, GetPostCountsOutput, GetPostEngagementsOutput, GetPostsOutput,
    GetUserFeedOutput, SearchPostsOutput, SearchRecentPostsOutput,
};

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum ToolSuccess {
    GetPosts(GetPostsOutput),
    GetPostMetrics(GetPostMetricsOutput),
    SearchRecentPosts(SearchRecentPostsOutput),
    SearchAllPosts(SearchPostsOutput),
    GetPostCounts(GetPostCountsOutput),
    GetUsers(UsersOutput),
    SearchUsers(UsersOutput),
    GetUserFeed(GetUserFeedOutput),
    GetPostEngagements(GetPostEngagementsOutput),
    GetRelationships(UsersOutput),
    GetLists(ListsOutput),
    GetSpaces(SpacesOutput),
    GetCommunities(CommunitiesOutput),
    GetTrends(TrendsOutput),
    GetDms(DmsOutput),
    GetMedia(MediaOutput),
    CreatePost(CreatePostOutput),
    ManagePost(ManagePostOutput),
    ManageRelationship(ManageRelationshipOutput),
    ManageList(ManageListOutput),
    ManageDm(ManageDmOutput),
    ManageMedia(ManageMediaOutput),
    CreateBookmarkFolder(CreateBookmarkFolderOutput),
}
