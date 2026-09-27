# Code signing policy

BRINEWAKE's macOS builds are signed with the maintainer's Apple Developer ID
and notarized by Apple.

The Windows builds are not code-signed yet, so Windows SmartScreen warns
before their first run. The Windows release binaries are already built by
this repository's GitHub Actions workflow from a tagged commit, so that a
code signing service can sign them later.

## Team roles

| Role | Who |
|---|---|
| Committers and reviewers | [halfprice06](https://github.com/halfprice06) (Daniel Price) |
| Approvers | [halfprice06](https://github.com/halfprice06) (Daniel Price) |

Every change reaches the main branch through a commit by a committer. Once
the Windows builds are signed, each signing request will need an approver's
approval.

## What is signed

The macOS launcher and game apps. Once Windows signing starts, the Windows
launcher (`BRINEWAKE.exe`) and game (`brinewake.exe`) built from this
repository, and nothing else.

Independently of Windows and Apple signing, every release manifest the
launcher installs from is signed with an ed25519 key held by the maintainer;
the launcher refuses any update whose signature, size or hash does not match.

## Privacy

BRINEWAKE has no accounts and collects no data. The launcher contacts
danprice.ai and GitHub to check for updates and download them. Online play
connects directly to the other players, asks public STUN servers (Google and
Cloudflare) for your public address, and may ask your router to open a port
(UPnP). Nothing else is sent anywhere.
