use argh::FromArgs;
use git2::{Oid, Repository};

#[derive(FromArgs)]
/// Arguments to pass to the rename utility
struct GitRenameStash {
    /// optional repository
    #[argh(option, default = "String::from(\".\")", short = 'r')]
    repository: String,

    /// the new message
    #[argh(option, short = 'm')]
    message: String,

    /// the stash whose message we intent to edit
    #[argh(positional, greedy)]
    stash: usize,
}

fn main() -> Result<(), git2::Error> {
    let args: GitRenameStash = argh::from_env();
    let stash_index = args.stash;
    let new_message = args.message;
    let repository = args.repository;

    let mut repo = Repository::open(repository)?;

    // get target stash
    let mut target_stash: Option<(usize, String, Oid)> = None;
    let stash_cb = |index: usize, message: &str, id: &Oid| -> bool {
        if index == stash_index {
            target_stash = Some((index, message.to_string(), *id));
            false
        } else {
            true
        }
    };
    repo.stash_foreach(stash_cb)?;

    let (index, _old_message, commit_id) =
        target_stash.ok_or_else(|| git2::Error::from_str("Stash not found"))?;

    let commit = repo.find_commit(commit_id)?;
    let tree = commit.tree()?;
    let parents = commit
        .parents()
        .map(|p| repo.find_commit(p.id()))
        .collect::<Result<Vec<_>, _>>()?;
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

    let new_commit = repo.commit(
        None,
        &commit.author(),
        &commit.committer(),
        &new_message,
        &tree,
        &parent_refs,
    )?;

    // let us work with `repo` in peace
    drop(commit);
    drop(tree);
    drop(parents);

    // drop existing stash
    repo.stash_drop(index)?;

    // update stash references
    // NOTE: the log messages of the stash reflog default to the commit messages of each stash
    repo.reference("refs/stash", new_commit, true, &new_message)?;

    Ok(())
}
