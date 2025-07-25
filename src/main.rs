use git2::{Oid, Repository};

fn main() -> Result<(), git2::Error> {
    let mut repo = match Repository::init(".") {
        Ok(repo) => repo,
        Err(e) => panic!("failed to init: {}", e),
    };

    // another comment
    let mut stashes = Vec::new();

    let stash_cb = |index: usize, message: &str, id: &Oid| -> bool {
        stashes.push((index, message.to_string(), *id));
        true
    };

    repo.stash_foreach(stash_cb)?;

    for (index, _message, id) in stashes {
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
            format!(
                "Stash number {}",
                char::from_u32(index as u32 + 97).unwrap_or('?')
            )
            .as_str(),
            &tree,
            &parent_refs,
        )?;

        // let us work with repo in peace
        drop(commit);
        drop(tree);
        drop(parents);

        // drop existing stash
        repo.stash_drop(index)?;

        // update stash references
        repo.reference("refs/stash", new_commit, true, "Updated stash message")?;
    }

    repo.stash_foreach(|index: usize, message: &str, id: &Oid| -> bool {
        println!("stash: {}, {}, {}", index, message, id);
        true
    })?;

    Ok(())
}
