# Publishing

## One-time setup

### 1. Dedicated AUR SSH key

Use a separate key that only works for the AUR, protected by a passphrase:

```sh
ssh-keygen -t ed25519 -C "aur" -f ~/.ssh/aur
```

Add to `~/.ssh/config`:

```
Host aur.archlinux.org
    User aur
    IdentityFile ~/.ssh/aur
    IdentitiesOnly yes
```

`IdentitiesOnly` makes ssh offer only this key to the AUR and never your GitHub key.

### 2. AUR account

1. Register at https://aur.archlinux.org and enable two-factor authentication.
2. **My Account → SSH Public Key**: paste `~/.ssh/aur.pub`.
3. First connection: compare the host key fingerprint with the ones listed on the AUR home page before typing `yes`.
4. Test: `ssh aur@aur.archlinux.org help`

### 3. GitHub

- Enable two-factor authentication.
- Protect `v*` tags: **Settings → Rules → Rulesets → New tag ruleset**, target `v*`, restrict creation and deletion to yourself.
- The release workflow only needs `contents: write`; keep it that way.

### 4. Tools

```sh
sudo pacman -S --needed pacman-contrib namcap devtools
```

## Every release

```sh
# 1. Version
#    Cargo.toml: version = "X.Y.Z"
#    data/io.github.GreisMicha.downmark.metainfo.xml: add <release version="X.Y.Z" date="YYYY-MM-DD">
cargo build && cargo test && git commit -am "Release X.Y.Z" && git push

# 2. Tag (message = short summary, shown on top of the release notes)
git tag -a vX.Y.Z -m "What changed, in one or two sentences."
git push origin vX.Y.Z
gh run watch      # wait for the Release workflow

# 3. PKGBUILD
cd packaging
sed -i 's/^pkgver=.*/pkgver=X.Y.Z/; s/^pkgrel=.*/pkgrel=1/' PKGBUILD
updpkgsums                          # downloads the tag tarball, writes sha256sums
extra-x86_64-build                  # clean chroot build, catches missing deps
namcap PKGBUILD *.pkg.tar.zst
makepkg --printsrcinfo > .SRCINFO
git commit -am "PKGBUILD X.Y.Z" && git push

# 4. AUR
git clone ssh://aur@aur.archlinux.org/downmark.git ~/aur/downmark   # first time only
cp PKGBUILD .SRCINFO ~/aur/downmark/
cd ~/aur/downmark && git add PKGBUILD .SRCINFO && git commit -m "Update to X.Y.Z" && git push
```

## Rules

- Never upload anything but `PKGBUILD` and `.SRCINFO` to the AUR repo.
- Always build from the tagged GitHub tarball with a pinned `sha256sums`, never from a branch.
- Review `git diff` in the AUR repo before every push.
- Only bump `pkgrel` (not `pkgver`) for packaging-only fixes.
- Lost or leaked key: remove it in **My Account** on the AUR and generate a new one.
