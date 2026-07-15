# ZFS → BTRFS/LUKS migration guide

Status: **planning document, nothing here has been applied yet**. This is the
agreed design + runbook for moving `hallandren` and `luthadel` off ZFS native
encryption onto disko-declared LUKS2 + BTRFS, with hibernation-capable
encrypted swap, a single password prompt at boot, and root still on tmpfs
(impermanence via `preservation`, unchanged).

## Why

- ZFS native encryption works, but isn't in-tree, and ZFS swap (zvol or an
  externally-encrypted partition with `randomEncryption`) doesn't support
  hibernation — this is why swap today uses `randomEncryption = true`,
  which by construction throws its key away every boot and can never resume
  a hibernation image.
- BTRFS + LUKS2 is in-tree, well trodden, and a raw LUKS-backed swap volume
  with a persistent key supports hibernation natively.

## Current state (as measured 2026-07-15)

| | hallandren | luthadel |
|---|---|---|
| Disks | 2× NVMe, ~931.5 GiB each, **striped** (no redundancy) `pool0` | 1× NVMe, ~931.5 GiB |
| RAM | 32 GiB now, **planned upgrade to 64 GiB** | 16 GiB |
| Partition layout | ESP(1G, disk0 only) + swap(16G×2, `randomEncryption`) + zfs(~914.5G×2) | ESP(1G) + swap(16G, `randomEncryption`) + zfs(~914.5G) |
| Pool usage | `nix` 77G, `persist` ~40G *(just cleaned from 600G+ by de-duplication — `zfs list` still shows the stale 646G figure until old snapshots pinning those blocks are destroyed)*, `cache` 433G (mostly the Steam library), `log` 55M | pool 174G used, 268G available |
| Free space on disks | **~1.7 MiB per disk** — partition tables are fully allocated, no room to carve a new layout in place | same — fully allocated |

Two consequences of this:

1. **hallandren can be migrated with no external backup drive at all.** Once
   the stale persist snapshots are dropped, live pool usage drops to roughly
   77 + 40 + 433 ≈ **550 GiB**, which fits on *one* of the two ~915 GiB
   disks. ZFS's device-removal feature can evacuate the second disk's vdev
   onto the first while the pool stays online, freeing that whole disk to
   build the new layout on — no other storage needed. See the hallandren
   runbook below.
2. **luthadel has no such trick** — it's a single, fully-partitioned disk, so
   its ~200–300 GiB working set has to be staged somewhere else temporarily.
   hallandren (which will still be on its old, untouched ZFS pool with
   ~604 GiB free at that point) is the natural staging target over the LAN —
   migrate luthadel first, using hallandren as scratch space, then migrate
   hallandren using the in-place trick above.

## Target architecture

Both hosts get: `GPT` → `ESP` (unencrypted `/boot`, vfat, unchanged) + one
"data" partition per disk → `LUKS2` → `LVM` → LVs for `swap` and `root`
(BTRFS, subvolumes for `/nix`, `/persist`, `/var/log`). Root itself stays
`tmpfs`, mounted the same way it is today; only what's *underneath*
`/nix` `/persist` `/var/log` `/cache` changes.

```
                 ┌─────────────────────────────────────────┐
   disk0         │ ESP (1G, vfat, /boot)                    │
   (root disk)   ├───────────────────────────────────────────
                 │ LUKS "crypt0"  ← interactive passphrase  │
                 │   └── LVM vg0                            │
                 │         ├── lv "swap"  (raw, resumeDevice)
                 │         └── lv "root"  (BTRFS)           │
                 │               ├── @nix     → /nix        │
                 │               ├── @persist → /persist    │
                 │               ├── @log     → /var/log    │
                 │               └── @keys    → /keys        │
                 └─────────────────────────────────────────┘

   disk1         ┌─────────────────────────────────────────┐
   (hallandren   │ LUKS "crypt1"  ← unlocked automatically  │
    only)        │   via keyfile at /keys/crypt1.key        │
                 │   (only reachable once disk0 is open)    │
                 │         └── BTRFS → /cache                │
                 └─────────────────────────────────────────┘
```

luthadel has only `disk0` — no `crypt1`, no `@keys`, no chaining needed; it's
a plain single-LUKS layout.

### Getting to exactly one password prompt

LUKS is per-block-device, unlike ZFS's pool-wide encryption, so two physical
disks under one prompt needs one of them to unlock *without* asking. NixOS's
systemd-stage-1 crypttab supports this directly: `boot.initrd.luks.devices.<name>.keyFile`
can point at a path, and `keyFileTimeout` tells it to keep retrying (rather
than fail immediately) until that path appears. So:

1. `crypt0` is unlocked interactively (the one prompt you type).
2. Its `@keys` BTRFS subvolume is mounted early in the initrd (`neededForBoot = true`,
   same mechanism already used for `/persist`/`/cache`/`/var/log` today) and
   holds a random 4096-bit keyfile for `crypt1`, mode `0400`, root-only.
3. `crypt1`'s crypttab entry has `keyFile = "/keys/crypt1.key"` and
   `keyFileTimeout = 10`; systemd polls for the file, finds it once `@keys`
   is mounted, and opens `crypt1` with no further input.

No custom systemd unit ordering is required — `keyFileTimeout` is exactly
built for this "keyfile lives on another device that isn't ready yet"
situation. `@keys` is deliberately its own subvolume, not backed up via
`preservation`'s data-dir (nothing about it should ever leave the machine).

This chaining mechanism is independent of what disk1 is *used for* — it
would work identically if disk1 were merged into disk0 via RAID0 instead of
kept separate. That choice is a separate question, addressed next.

### RAID0 vs. separate filesystems per disk

You asked for this explicitly, since your Steam library (`/cache`, ~433G)
is the main occupant of disk1 and could live on either topology.

**RAID0 (mdadm, striping both disks into one pool/filesystem)**

- \+ One pooled capacity — `/nix` `/persist` `/cache` could all grow into
  either disk without you ever choosing which disk something lives on
  (matches today's ZFS stripe behavior).
- \+ Possible throughput gain for large sequential I/O that spans both
  drives (e.g. huge single-file copies, video scratch space).
- − Failure blast radius = both disks: lose either one, lose *everything*
  (persist, nix, cache) — identical to today's exposure, not an improvement
  despite the migration effort.
- − For a Steam library specifically, striping rarely matters in practice:
  game loading is dominated by many small/medium random reads and CPU-side
  decompression, not raw sequential throughput. A single Samsung 980 or 970
  EVO Plus (~3.5 GB/s each) already exceeds what a game engine can consume;
  you're unlikely to notice faster load times from striping.
- − mdadm RAID0 arrays are inflexible later: you can't remove or resize a
  member disk without rebuilding the whole array.

**Separate filesystems per disk (recommended — what's diagrammed above)**

- \+ Failure isolation: if the Steam-library disk dies, you lose reinstallable
  game data only; `persist`/`nix` on disk0 are untouched. This is a genuine
  improvement over today's stripe, not just parity.
- \+ Disk1 can be wiped, resized, or replaced independently at any time —
  no array to rebuild, no impact on the bootable root.
- \+ Simpler failure recovery: two ordinary filesystems, no array state to
  reason about.
- − No capacity pooling: if `/cache` ever needs more than one disk's worth
  of space you're resizing/adding a disk rather than letting the pool
  absorb it. Not a near-term concern (433G used of 915G available).
- − Slightly more one-time disko/initrd config (the keyfile chaining above),
  though this is a fixed setup cost, not ongoing complexity.

**Recommendation:** separate filesystems, per the diagram — it turns your
current single point of total failure into "lose the Steam library" instead
of "lose everything," at effectively no performance cost for this workload,
and you still get one password prompt via keyfile chaining. If you later
want pooled/striped capacity, disk1 can be converted to an mdadm member
without touching disk0/root — a contained change, not a redo of this whole
migration.

### Swap / hibernation sizing

Swap is a plain LVM logical volume, not a BTRFS swapfile — this avoids ever
having to track/recompute `resume_offset` (a real maintenance trap: it
silently breaks if `btrfs balance`/`defrag` ever relocates the swapfile). A
raw LV is auto-detected as a resume device by NixOS's stage-1 with no extra
kernel params, and is trivially resizable later (`lvextend` + `mkswap`) if
you outgrow it.

- **luthadel**: 16 GiB RAM → **20 GiB swap LV** (RAM + margin).
- **hallandren**: sized for the *planned* 64 GiB upgrade, not the current
  32 GiB, per your ask → **72 GiB swap LV** now. This "wastes" ~40G of disk
  relative to today's actual need, but avoids a second disruptive resize
  later; if you'd rather not commit the space yet, LVM makes shrinking this
  after the fact trivial too — say the word and I'll size it for 32G+margin
  instead.

### Carrying over scrub / auto-snapshot / trim

| ZFS today | BTRFS equivalent |
|---|---|
| `services.zfs.autoScrub` (monthly) | `services.btrfs.autoScrub` — built into nixpkgs, same shape (`enable`, `fileSystems`, `interval`) |
| `services.zfs.autoSnapshot` (frequent/hourly/daily/weekly) on `/persist` | `services.snapper` — `TIMELINE_CREATE`/`TIMELINE_LIMIT_HOURLY`/`_DAILY`/`_WEEKLY`/`_MONTHLY` per subvolume config, closest like-for-like to zfs-auto-snapshot's retention knobs |
| `services.zfs.trim` (weekly) | Already redundant: both hosts already set `services.fstrim.enable = true`, which works at the block-device level regardless of filesystem. Just needs `settings.allowDiscards = true` on both LUKS containers so TRIM passes through the encryption layer to the SSDs. |

Example for the `filesystem` module on each host (replaces `zfs.nix`):

```nix
{ config, ... }:
{
  services.btrfs.autoScrub = {
    enable = true;
    interval = "monthly";
    # fileSystems defaults to every mounted btrfs fs; leave unset unless
    # you want to exclude one.
  };

  services.snapper.configs.persist = {
    SUBVOLUME = "/persist";
    TIMELINE_CREATE = true;
    TIMELINE_CLEANUP = true;
    TIMELINE_LIMIT_HOURLY = 24;
    TIMELINE_LIMIT_DAILY = 7;
    TIMELINE_LIMIT_WEEKLY = 4;
    TIMELINE_LIMIT_MONTHLY = 0;
  };
}
```

## disko modules

### hallandren

`hosts/hallandren/filesystem/disks.nix`:

```nix
let
  disk0 = "/dev/disk/by-id/nvme-Samsung_SSD_980_1TB_S649NU0W832448H";
  disk1 = "/dev/disk/by-id/nvme-eui.0025385501b07a5d";

  boot-size = "1G";
  swap-size = "72G"; # sized for a future 64G RAM upgrade, see fs.md
in
{
  disko.devices.disk = {
    disk0 = {
      type = "disk";
      device = disk0;
      content = {
        type = "gpt";
        partitions = {
          ESP = {
            size = boot-size;
            type = "EF00";
            content = {
              type = "filesystem";
              format = "vfat";
              mountpoint = "/boot";
              mountOptions = [
                "nofail"
                "umask=0077"
              ];
            };
          };
          crypt0 = {
            size = "100%";
            content = {
              type = "luks";
              name = "crypt0";
              settings.allowDiscards = true;
              # askPassword = true is the default when no keyFile/passwordFile is set
              content = {
                type = "lvm_pv";
                vg = "vg0";
              };
            };
          };
        };
      };
    };

    disk1 = {
      type = "disk";
      device = disk1;
      content = {
        type = "gpt";
        partitions = {
          crypt1 = {
            size = "100%";
            content = {
              type = "luks";
              name = "crypt1";
              settings = {
                allowDiscards = true;
                keyFile = "/keys/crypt1.key";
                keyFileTimeout = 10;
              };
              initrdUnlock = true;
              content = {
                type = "btrfs";
                extraArgs = [ "-f" ];
                mountpoint = "/cache";
                mountOptions = [
                  "compress=zstd"
                  "noatime"
                ];
              };
            };
          };
        };
      };
    };
  };

  disko.devices.lvm_vg.vg0 = {
    type = "lvm_vg";
    lvs = {
      swap = {
        size = swap-size;
        content = {
          type = "swap";
          resumeDevice = true;
          discardPolicy = "both";
        };
      };
      root = {
        size = "100%FREE";
        content = {
          type = "btrfs";
          extraArgs = [ "-f" ];
          subvolumes = {
            "/nix" = {
              mountpoint = "/nix";
              mountOptions = [ "compress=zstd" "noatime" ];
            };
            "/persist" = {
              mountpoint = "/persist";
              mountOptions = [ "compress=zstd" "noatime" ];
            };
            "/log" = {
              mountpoint = "/var/log";
              mountOptions = [ "compress=zstd" "noatime" ];
            };
            "/keys" = {
              mountpoint = "/keys";
              mountOptions = [ "noatime" ];
            };
          };
        };
      };
    };
  };
}
```

`hosts/hallandren/filesystem/btrfs.nix` (replaces `zfs.nix` + `zpool.nix`):

```nix
{
  fileSystems."/" = {
    device = "none";
    fsType = "tmpfs";
    options = [
      "defaults"
      "size=2G"
      "mode=755"
    ];
  };

  # mark preserved/always-on filesystems as needed for boot
  fileSystems."/keys".neededForBoot = true;
  fileSystems."/cache".neededForBoot = true;
  fileSystems."/persist".neededForBoot = true;
  fileSystems."/var/log".neededForBoot = true;

  boot.initrd.systemd.enable = true;
  preservation.enable = true;
  preservation.preserveAt.data-dir.persistentStoragePath = "/persist";
  preservation.preserveAt.state-dir.persistentStoragePath = "/cache";

  services.btrfs.autoScrub = {
    enable = true;
    interval = "monthly";
  };

  services.snapper.configs.persist = {
    SUBVOLUME = "/persist";
    TIMELINE_CREATE = true;
    TIMELINE_CLEANUP = true;
    TIMELINE_LIMIT_HOURLY = 24;
    TIMELINE_LIMIT_DAILY = 7;
    TIMELINE_LIMIT_WEEKLY = 4;
    TIMELINE_LIMIT_MONTHLY = 0;
  };
}
```

`hosts/hallandren/filesystem/default.nix`:

```nix
{ inputs, ... }:
{
  imports = [
    inputs.disko.nixosModules.disko
    ./disks.nix
    ./btrfs.nix
  ];
}
```

Delete `zfs.nix` and `zpool.nix`; remove the ZFS `kernelPackages` pinning
logic, `boot.zfs.*`, `supportedFilesystems = [ "zfs" ]` from wherever they
live in the host/system modules.

### luthadel

Same shape, single disk, no chaining:

```nix
let
  disk0 = "/dev/disk/by-id/<luthadel-nvme-id>"; # confirm with `ls -l /dev/disk/by-id`
  boot-size = "1G";
  swap-size = "20G"; # 16G RAM + margin
in
{
  disko.devices.disk.disk0 = {
    type = "disk";
    device = disk0;
    content = {
      type = "gpt";
      partitions = {
        ESP = {
          size = boot-size;
          type = "EF00";
          content = {
            type = "filesystem";
            format = "vfat";
            mountpoint = "/boot";
            mountOptions = [ "nofail" "umask=0077" ];
          };
        };
        crypt0 = {
          size = "100%";
          content = {
            type = "luks";
            name = "crypt0";
            settings.allowDiscards = true;
            content = {
              type = "lvm_pv";
              vg = "vg0";
            };
          };
        };
      };
    };
  };

  disko.devices.lvm_vg.vg0 = {
    type = "lvm_vg";
    lvs = {
      swap = {
        size = swap-size;
        content = {
          type = "swap";
          resumeDevice = true;
          discardPolicy = "both";
        };
      };
      root = {
        size = "100%FREE";
        content = {
          type = "btrfs";
          extraArgs = [ "-f" ];
          subvolumes = {
            "/nix" = {
              mountpoint = "/nix";
              mountOptions = [ "compress=zstd" "noatime" ];
            };
            "/persist" = {
              mountpoint = "/persist";
              mountOptions = [ "compress=zstd" "noatime" ];
            };
            "/log" = {
              mountpoint = "/var/log";
              mountOptions = [ "compress=zstd" "noatime" ];
            };
          };
        };
      };
    };
  };
}
```

`btrfs.nix` is the same as hallandren's, minus the `/keys`/`/cache`
`neededForBoot` lines and the `preservation.preserveAt.state-dir` mapping
(check whether luthadel currently uses `/cache` the same way — if so it
needs its own `disko.devices.disk...` for it too, since it only has one
disk here it would just be another BTRFS subvolume on `root`, not a
separate device).

## Migration runbook

Do **luthadel first** (single disk, smaller, rehearses disko/nixos-install/
preservation/hibernation without the keyfile-chaining complexity), then
**hallandren** (uses the in-place trick, no external storage).

### 0. Both hosts, before touching anything

- Commit the new `disko.nix`/`btrfs.nix` modules on a branch; `nix flake check`
  and `nixos-rebuild build --flake .#<host>` (build only, don't switch) to
  confirm they evaluate.
- `git log`/`git status` clean; note down the currently-booted generation
  in case you need to roll back the *config* (this doesn't help once disks
  are wiped — that's what the backup step is for).
- Double check `/dev/disk/by-id/...` names for both machines with
  `ls -l /dev/disk/by-id/ | grep nvme` — don't rely on `/dev/nvme0n1` device
  ordering.

### 1. luthadel

1. **Stage a backup to hallandren** (still on its old, untouched ZFS pool,
   with ~604G free): from luthadel,
   `rsync -aHAX --info=progress2 /persist/ /cache/ /var/log/ nixos@hallandren:/cache/luthadel-migration-backup/`
   (or `zfs send` if you'd rather keep it dataset-shaped — plain rsync is
   simpler to restore from later). Verify the copy (`du -sh` comparison,
   spot-check a few files) before proceeding — this is the only copy that
   will exist once luthadel's disk is wiped.
2. From hallandren (which already has `nixos-anywhere` referenced in
   `hosts/luthadel/... ` comments), kexec into a temporary installer
   environment on luthadel:
   ```
   nix run github:nix-community/nixos-anywhere -- \
     --kexec-only --flake /persist/nixos#luthadel --target-host nixos@<luthadel-ip>
   ```
   (or boot luthadel from a physical NixOS installer USB if you'd rather not
   kexec a laptop you're sitting at.)
3. In that temporary environment, run `disko` against the new
   `disko.devices` for luthadel — this **destroys the old ZFS partition**,
   so only proceed once step 1's backup is verified:
   ```
   nix run github:nix-community/disko -- --mode disko --flake /persist/nixos#luthadel
   ```
   This prompts once for the LUKS passphrase (format + first unlock) and
   leaves the new layout mounted at `/mnt`.
4. Restore the data: `rsync -aHAX nixos@hallandren:/cache/luthadel-migration-backup/persist/ /mnt/persist/`
   (and the same for `/mnt/var/log`; `/nix` doesn't need restoring — it'll
   populate on install/first build. `/cache`'s equivalent content, if any,
   restores into `/mnt/persist` or wherever it's rooted on this host.)
5. `nixos-install --root /mnt --flake /persist/nixos#luthadel` (installs the
   bootloader against the new ESP + ships the built closure into the new
   `/nix`).
6. Reboot into the real system. Verify: boots with one password prompt,
   `swapon --show` lists the new LV, `systemctl status snapper-timeline.timer`
   and the btrfs scrub timer are active, `zramctl`/`free -h` look sane.
7. Test hibernation (`systemctl hibernate`), confirm it resumes cleanly.
8. Once confirmed stable, delete `nixos@hallandren:/cache/luthadel-migration-backup/`.

### 2. hallandren

Do this **entirely within one offline session** (kexec or USB) to avoid any
drift between the "current data" snapshot and what actually lands on the
new filesystem — don't split the ZFS surgery and the disko/copy steps
across a reboot with normal usage in between.

1. **While still normally booted**, reclaim the already-deleted persist
   data pinned by old auto-snapshots:
   ```
   zfs list -t snapshot -r pool0/root/persist
   zfs destroy -r pool0/root/persist@<oldest>%<newest>   # or destroy individually
   zfs list pool0/root/persist    # confirm ~40G, not ~640G
   ```
2. **Still normally booted**, evacuate disk1's vdev onto disk0 — this is a
   supported *online* ZFS operation, safe to run on the live system (it'll
   take a while and add some I/O load; the pool stays usable throughout):
   ```
   zpool status pool0   # note the exact vdev name for disk1's partition
   zpool remove pool0 nvme-eui.0025385501b07a5d-part2
   zpool status pool0   # watch "remove: ... in progress" to completion
   ```
   Confirm afterwards that `zpool status` shows only disk0's vdev and that
   `df`/`zfs list` usage (~550G) comfortably fits disk0's ~915G capacity.
3. **Now** kexec (or boot USB) into a temporary environment — the running
   system is about to go away for a while, this is a maintenance window:
   ```
   nix run github:nix-community/nixos-anywhere -- \
     --kexec-only --flake /persist/nixos#hallandren --target-host nixos@localhost
   ```
   (or physical USB if kexec against localhost is awkward — either way, get
   to a shell where nothing from the running OS is mounted anymore.)
4. Import the (now single-vdev) old pool read-only to pull data from:
   ```
   zpool import -f -o readonly=on -R /mnt-old pool0
   ```
5. Run `disko` for hallandren's new layout. Since `disko.devices.disk`
   covers both disk0 and disk1, this wipes both — that's fine, disk1 was
   only ever holding what step 2 already evacuated:
   ```
   nix run github:nix-community/disko -- --mode disko --flake /persist/nixos#hallandren
   ```
   This prompts once for `crypt0`'s passphrase and leaves the new layout
   mounted at `/mnt`, including the still-empty `/mnt/keys`.
6. Generate and place `crypt1`'s keyfile, then format disk1's LUKS with it
   directly (steps folded together so disk1 never needs a placeholder key):
   ```
   head -c 4096 /dev/urandom > /mnt/keys/crypt1.key
   chmod 0400 /mnt/keys/crypt1.key
   cryptsetup luksFormat /dev/disk/by-id/<disk1-id>-part1 --key-file /mnt/keys/crypt1.key
   cryptsetup open /dev/disk/by-id/<disk1-id>-part1 crypt1 --key-file /mnt/keys/crypt1.key
   mkfs.btrfs -f /dev/mapper/crypt1
   mount /dev/mapper/crypt1 /mnt/cache
   ```
   (If you'd rather have disko drive this step too instead of doing it by
   hand, order the initial `disko` run to target disk0 only, populate the
   keyfile, then run disko again scoped to disk1 — either approach works,
   this is just the manual fallback.)
7. Copy data from the imported old pool into the new layout:
   ```
   rsync -aHAX --info=progress2 /mnt-old/persist/ /mnt/persist/
   rsync -aHAX --info=progress2 /mnt-old/cache/   /mnt/cache/
   rsync -aHAX --info=progress2 /mnt-old/var/log/ /mnt/var/log/
   ```
   (`/nix` doesn't need copying — `nixos-install` populates it.)
8. `zpool export pool0` (done with the old pool for good).
9. `nixos-install --root /mnt --flake /persist/nixos#hallandren`.
10. Reboot. Verify: one password prompt, `crypt1` auto-unlocks without being
    asked, `lsblk` shows both LUKS containers open, `swapon --show`,
    `systemctl hibernate` + resume test, snapper/scrub timers active,
    `cryptsetup luksDump /dev/disk/by-id/<disk0-id>-part2 | grep Discards`
    and same for disk1 to confirm `allowDiscards` took, `fstrim -av` runs
    clean.

### Cleanup (both hosts, after each is verified stable for a few days)

- Remove the `zfs` flake input if nothing else in the repo uses it.
- `networking.hostId` can stay — it's harmless once ZFS is gone, just
  vestigial (it was originally there to guard against double pool imports).
- Delete this file's "planning document" caveat once both hosts are done, or
  fold the finished layout description into each host's own README/comments.

## Risk notes

- Step 2 of hallandren's runbook (`zpool remove`) is the only "online"
  destructive-adjacent step — it's a well-supported ZFS feature but leaves a
  permanent indirect-mapping table on the pool. That's irrelevant here since
  the whole pool is destroyed minutes later, but don't reuse this pool
  long-term after a partial removal for unrelated reasons.
- Everything from "run `disko`" onward is a point of no return for that
  host's old filesystem. Don't proceed past that point without a verified
  backup (luthadel) or a verified `zpool remove` completion + rsync source
  (hallandren).
- Test hibernation on each host *before* considering the migration done —
  it's the main feature being migrated for, and the failure mode (corrupt
  resume, black screen) is much easier to debug with the old system still
  one `git revert` + reinstall away than after weeks of drift.
