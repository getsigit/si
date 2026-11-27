# siGit for Claude

This plugin connects Claude to your account on [siGit](https://sigit.si), the Git hosting
service for code and open-weights models. Once it is connected, you can ask Claude about the
repositories you can see on sigit.si, have it read and search their code, and have it open
issues and pull requests for you.

## Signing in

The first time Claude uses a siGit tool, it asks you to sign in. A browser window opens on
sigit.si, where you sign in to your siGit account and approve Claude's access. Claude gets
read access to your profile and to the repositories you can see, and permission to act for
you on the tools below. Nothing is shared until you approve. To disconnect Claude, revoke it
under Settings, Authorized applications on sigit.si. Its token stops working immediately.

## Tools

| Tool | What it does |
| --- | --- |
| `list_repositories` | Lists the repositories you can see, optionally filtered by name |
| `get_file_contents` | Reads a file at a branch, tag or commit |
| `search_code` | Searches one repository's code for a string |
| `list_issues`, `get_issue` | Lists issues and reads one with its comments |
| `create_issue` | Opens an issue |
| `list_pull_requests`, `get_pull_request` | Lists pull requests and reads one with its comments and diff |
| `create_pull_request` | Opens a pull request between two branches |
| `add_issue_comment` | Comments on an issue or pull request |
| `web_search` | Searches the web |

`create_issue`, `create_pull_request` and `add_issue_comment` change things on sigit.si. The
other tools only read.

## Your data

The tools read and write data in your siGit account on sigit.si. What they return is sent to
Claude and handled under Anthropic's terms. `web_search` sends your search query to Brave
Search, which returns the results; Brave does not receive your account details. siGit keeps
your repositories, issues and pull requests until you delete them or your account. The
[siGit privacy policy](https://sigit.si/privacy) covers all of this in full, and questions
about your data go to privacy@sigit.si.

## License

Apache-2.0
