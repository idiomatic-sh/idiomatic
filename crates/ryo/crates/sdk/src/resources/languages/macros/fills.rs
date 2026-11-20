/// Declares how a language fills a relation for the engine, from its per-file view: the one way an item implements
/// `ryo::languages::FillItem`. The language's `Fillers` function for the relation calls it with `fillers::filled::<Item>`.
///
/// ```text
/// ryo::fills! {
///     Item => languages::Relation, walk(Language) |file, words, rows| {
///         // `file`: &mut the language's `Fillers::File` (its parse in place, a typed node per id, its model on first need);
///         // `words`: the file's words (`facts::intern` a built text); `rows`: the relation's rows, `rows.write(Row { .. })`,
///         // `rows.partial(..)` for a part it cannot read.
///     }
/// }
/// ```
///
/// A filler writes every row of the relation for the file, once: the engine calls it the first time an enabled program reads the
/// relation in the file, and never again for that file.
///
/// The language's `Fillers` function of the relation runs the item: `fn syntax_error(file, words, rows) -> Fill {
/// fillers::filled::<SyntaxErrors>(file, words, rows) }`, as the language crate's fixture language does
/// (`fillers::fixtures::Walking`).
#[macro_export]
macro_rules! fills {
    (
        $item:ident => $relation:ty, walk ($walk:ty)
        |$file:ident, $words:ident, $rows:ident|
        { $($body:tt)* }
    ) => {
        impl $crate::languages::FillItem for $item {
            type Walk = $walk;
            type Relation = $relation;

            fn fill(
                $file: &mut <$walk as $crate::languages::Fillers>::File<'_>,
                $words: &mut $crate::languages::FileWords,
                $rows: &mut dyn $crate::languages::Rows<<$relation as $crate::languages::CatalogRelation>::Row>,
            ) {
                $($body)*
            }
        }
    };
}
