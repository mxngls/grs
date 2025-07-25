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
    stash: u16,
}

fn main() -> Result<(), git2::Error> {
    let args: GitRenameStash = argh::from_env();
    let stash_index = args.stash;
    let new_message = args.message;
    let repository = args.repository;

    let mut repo = Repository::open(repository)?;

    // collect stashes
    let mut stashes = Vec::new();
    let stash_cb = |index: usize, message: &str, id: &Oid| -> bool {
        stashes.push((index, message.to_string(), *id));
        index != stash_index.into()
    };
    repo.stash_foreach(stash_cb)?;

    for (index, _message, id) in stashes {
        if index != stash_index.into() {
            continue;
        }

        let commit = repo.find_commit(id)?;
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
    }

    Ok(())
}
