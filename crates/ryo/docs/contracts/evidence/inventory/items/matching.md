# Items: matching

One row per concept: every struct, enum, trait, function and type alias written with a visibility other than private, as `visibility kind name`.

| concept | items | items defined |
|---|---|---|
| domain/globbing | 18 | pub(crate) enum ClassMembership; pub(crate) enum GlobMatch; pub(crate) enum Negation; pub(crate) enum Ruling; pub(crate) enum TextGlobProgram; pub(crate) function compile; pub(crate) function matches; pub(crate) function matches_compiled; pub(crate) struct Alternated; pub(crate) struct Byte; pub(crate) struct ClassBounds; pub(crate) struct ClassClose; pub(crate) struct GlobIndex; pub(crate) struct GlobPiece; pub(crate) struct GlobText; pub(crate) struct Letter; pub(crate) struct OwnedGlobText; pub(crate) struct Subject |
| domain/matching | 17 | pub(crate) enum RegexMatch; pub(crate) function compile_globs; pub(crate) function compile_regex; pub(crate) function match_globs; pub(crate) function matches_regex; pub(crate) function parse_glob; pub(crate) function visit_regex_captures; pub(crate) function visit_regex_matches; pub(crate) struct AuthoredIndices; pub(crate) struct AuthoredRegex; pub(crate) struct CaptureName; pub(crate) struct CompilerReason; pub(crate) struct GlobRefusal; pub(crate) struct Haystack; pub(crate) struct MatchSpan; pub(crate) struct PathGlob; pub(crate) struct RefusedGlob |
| inbound/use_cases/matching | 13 | pub enum MatchOutcome; pub enum MatchingError; pub struct CompiledGlobs; pub struct CompiledRegex; pub struct CompiledTextGlobs; pub struct GlobHits; pub struct GlobPattern; pub struct GroupName; pub struct MatchingUseCaseImpl; pub struct RegexSource; pub struct SearchText; pub struct TextSpan; pub trait MatchingUseCase |
