// Common git commands, grouped by what people are trying to do. Uses switch and restore, not checkout.

export interface GitCommand {
  command: string;
  description: string;
  /** Shown prominently: the command can lose work or rewrite shared history. */
  warning?: string;
}

export interface GitGroup {
  id: string;
  name: string;
  commands: GitCommand[];
}

const REWRITES = 'Rewrites commits. Avoid it on commits other people already have.';

export const gitGroups: GitGroup[] = [
  {
    id: 'setup',
    name: 'Set up',
    commands: [
      { command: 'git config --global user.name "Your Name"', description: 'Set the name recorded on your commits.' },
      { command: 'git config --global user.email "you@example.com"', description: 'Set the email address recorded on your commits.' },
      { command: 'git config --global init.defaultBranch main', description: 'Call the first branch of new repositories main.' },
      { command: 'git config --global core.editor "code --wait"', description: 'Choose the editor git opens for commit messages.' },
      { command: 'git config --global alias.st status', description: 'Add a shortcut, so git st runs git status.' },
      { command: 'git config --global --list', description: 'Show all your global settings.' },
    ],
  },
  {
    id: 'start',
    name: 'Start a repository',
    commands: [
      { command: 'git init', description: 'Turn the current folder into a new, empty repository.' },
      { command: 'git clone <url>', description: 'Copy a remote repository, with its history, into a new folder.' },
      { command: 'git clone --depth 1 <url>', description: 'Copy only the latest commit, which is much faster for a long history.' },
    ],
  },
  {
    id: 'everyday',
    name: 'Everyday work',
    commands: [
      { command: 'git status', description: 'See which files are changed, staged or untracked.' },
      { command: 'git add <file>', description: 'Stage a file, so its changes go in the next commit.' },
      { command: 'git add .', description: 'Stage every change in the current folder and below.' },
      { command: 'git add -p', description: 'Go through your changes piece by piece and choose which to stage.' },
      { command: 'git commit -m "Message"', description: 'Record the staged changes as a commit with a message.' },
      {
        command: 'git commit --amend',
        description: 'Change the last commit’s message, or add staged changes you forgot.',
        warning: REWRITES,
      },
      { command: 'git diff', description: 'Show changes you haven’t staged yet.' },
      { command: 'git diff --staged', description: 'Show the staged changes that will go in the next commit.' },
      { command: 'git rm <file>', description: 'Delete a file and stage the deletion.' },
      { command: 'git mv <old> <new>', description: 'Rename or move a file and stage the change.' },
    ],
  },
  {
    id: 'branches',
    name: 'Branches',
    commands: [
      { command: 'git branch', description: 'List local branches. The current one has a star.' },
      { command: 'git branch --all', description: 'List local branches and the remote-tracking ones.' },
      { command: 'git switch <branch>', description: 'Move to another branch.' },
      { command: 'git switch -c <branch>', description: 'Create a new branch and move to it.' },
      { command: 'git switch -c <branch> <commit>', description: 'Create a branch at an older commit, such as one found with git reflog.' },
      { command: 'git switch -', description: 'Go back to the branch you were on before.' },
      { command: 'git merge <branch>', description: 'Bring another branch’s commits into the current branch.' },
      { command: 'git rebase <branch>', description: 'Replay your commits on top of another branch, for a straight history.', warning: REWRITES },
      { command: 'git branch -m <new-name>', description: 'Rename the current branch.' },
      { command: 'git branch -d <branch>', description: 'Delete a branch that has been merged.' },
      {
        command: 'git branch -D <branch>',
        description: 'Delete a branch even if it hasn’t been merged.',
        warning: 'Commits only on that branch are left behind. git reflog can still find them for a while.',
      },
    ],
  },
  {
    id: 'remotes',
    name: 'Remotes',
    commands: [
      { command: 'git remote -v', description: 'List remotes and their URLs.' },
      { command: 'git remote add origin <url>', description: 'Connect the repository to a remote called origin.' },
      { command: 'git fetch', description: 'Download new commits and branches without changing your files.' },
      { command: 'git pull', description: 'Fetch, then merge the upstream changes into the current branch.' },
      { command: 'git pull --rebase', description: 'Fetch, then replay your local commits on top of the upstream ones.' },
      { command: 'git push', description: 'Send your commits to the remote.' },
      { command: 'git push -u origin <branch>', description: 'Push a new branch and remember where it goes, so plain git push works next time.' },
      {
        command: 'git push --force-with-lease',
        description: 'Replace the remote branch after rewriting history, but only if nobody else has pushed to it since.',
        warning: 'Overwrites the remote branch. Prefer it to --force, which doesn’t check.',
      },
      { command: 'git push origin --delete <branch>', description: 'Delete a branch on the remote.' },
    ],
  },
  {
    id: 'undo',
    name: 'Undoing things',
    commands: [
      {
        command: 'git restore <file>',
        description: 'Throw away the unstaged changes to a file.',
        warning: 'The changes are lost and can’t be recovered.',
      },
      { command: 'git restore --staged <file>', description: 'Unstage a file, keeping its changes in your working copy.' },
      { command: 'git restore --source <commit> <file>', description: 'Bring back a file as it was in an earlier commit.' },
      { command: 'git revert <commit>', description: 'Make a new commit that undoes an earlier one. Safe on shared branches.' },
      { command: 'git reset --soft HEAD~1', description: 'Undo the last commit and keep its changes staged.' },
      { command: 'git reset --mixed HEAD~1', description: 'Undo the last commit and keep its changes, unstaged. This is what plain git reset does.' },
      {
        command: 'git reset --hard HEAD~1',
        description: 'Undo the last commit and throw away its changes, along with any uncommitted work.',
        warning:
          'Uncommitted changes are lost for good. The commit itself can still be found with git reflog for a while. Run git status first.',
      },
      { command: 'git reflog', description: 'List everywhere HEAD has been, to find commits that seem lost after a reset or rebase.' },
    ],
  },
  {
    id: 'history',
    name: 'History and inspecting',
    commands: [
      { command: 'git log', description: 'Show the commit history of the current branch.' },
      { command: 'git log --oneline --graph --all', description: 'A compact history of every branch, drawn as a graph.' },
      { command: 'git log -p <file>', description: 'Show every change ever made to a file.' },
      { command: 'git log -S "text"', description: 'Find the commits that added or removed some text.' },
      { command: 'git show <commit>', description: 'Show what one commit changed.' },
      { command: 'git blame <file>', description: 'Show who last changed each line of a file, and in which commit.' },
      { command: 'git diff <branch-a>..<branch-b>', description: 'Compare two branches.' },
      { command: 'git shortlog -sn', description: 'Count commits per author.' },
      { command: 'git bisect start', description: 'Search through history, half at a time, for the commit that introduced a bug.' },
    ],
  },
  {
    id: 'stash',
    name: 'Stash',
    commands: [
      { command: 'git stash', description: 'Put uncommitted changes aside and leave a clean working copy.' },
      { command: 'git stash push -m "Message"', description: 'Stash changes with a name, so they are easy to find later.' },
      { command: 'git stash -u', description: 'Stash changes including untracked files.' },
      { command: 'git stash list', description: 'List your stashes.' },
      { command: 'git stash pop', description: 'Apply the latest stash and remove it from the list.' },
      { command: 'git stash apply', description: 'Apply the latest stash and keep it in the list.' },
      { command: 'git stash drop', description: 'Delete the latest stash without applying it.' },
    ],
  },
  {
    id: 'tags',
    name: 'Tags',
    commands: [
      { command: 'git tag', description: 'List the tags in this repository.' },
      { command: 'git tag -a v1.0.0 -m "Version 1.0.0"', description: 'Create an annotated tag, with a message, author and date, on the current commit.' },
      { command: 'git tag v1.0.0 <commit>', description: 'Create a lightweight tag on a given commit.' },
      { command: 'git push origin v1.0.0', description: 'Send one tag to the remote. Tags aren’t pushed by default.' },
      { command: 'git push --tags', description: 'Send all your tags to the remote.' },
      { command: 'git tag -d v1.0.0', description: 'Delete a tag locally.' },
      { command: 'git push origin --delete v1.0.0', description: 'Delete a tag on the remote.' },
    ],
  },
  {
    id: 'clean',
    name: 'Cleaning up',
    commands: [
      { command: 'git clean -n', description: 'Preview which untracked files git clean would delete.' },
      {
        command: 'git clean -fd',
        description: 'Delete untracked files and folders.',
        warning: 'Untracked files aren’t in git, so they can’t be recovered. Run git clean -n first.',
      },
      { command: 'git fetch --prune', description: 'Remove remote-tracking branches that were deleted on the remote.' },
      { command: 'git branch --merged', description: 'List branches already merged into the current one, which are safe to delete.' },
      { command: 'git gc', description: 'Compress and tidy up the repository’s internal files.' },
    ],
  },
];

/** Every word must appear in the command, its description or its group name. */
export function searchGit(query: string, groups: GitGroup[] = gitGroups): GitGroup[] {
  const words = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  if (!words.length) return groups;
  return groups
    .map((g) => ({
      ...g,
      commands: g.commands.filter((c) => {
        const hay = `${c.command} ${c.description} ${g.name}`.toLowerCase();
        return words.every((w) => hay.includes(w));
      }),
    }))
    .filter((g) => g.commands.length);
}
