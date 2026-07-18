# ZFS → BTRFS/LUKS migration guide (hallandren)

Status: **planning document, nothing here has been applied yet**. This is the
agreed design + runbook for moving `hallandren` off ZFS native encryption
onto disko-declared LUKS2 + BTRFS, with hibernation-capable encrypted swap, a
single password prompt at boot (eventually), and root still on tmpfs
(impermanence via `preservation`, unchanged).

`luthadel`'s migration is complete and has been removed from this document.

The migration is split into two phases:

- **Phase 1 (this round):** evacuate all ZFS data onto `disk1`, disko `disk0`
  into the new boot + LUKS + LVM + BTRFS layout, install onto `disk0`, then
  pull the data back from `disk1`. `disk1` itself is left exactly as it is
  today — still ZFS, still natively encrypted, unchanged mount behavior.
- **Phase 2 (later, only when more space is actually needed):** convert
  `disk1` to LUKS + LVM + BTRFS to match `disk0`, at which point ZFS can be
  fully retired from this host.

## Why

- ZFS native encryption works, but isn't in-tree, and ZFS swap (zvol or an
  externally-encrypted partition with `randomEncryption`) doesn't support
  hibernation — this is why swap today uses `randomEncryption = true`,
  which by construction throws its key away every boot and can never resume
  a hibernation image.
- BTRFS + LUKS2 is in-tree, well trodden, and a raw LUKS-backed swap volume
  with a persistent key supports hibernation natively.

## Current state (as measured 2026-07-15)

| | hallandren |
|---|---|
| Disks | 2× NVMe, ~931.5 GiB each, **striped** (no redundancy) `pool0` |
| RAM | 32 GiB now, **planned upgrade to 64 GiB** |
| Partition layout | ESP(1G, disk0 only) + swap(16G×2, `randomEncryption`) + zfs(~914.5G×2) |
| Pool usage | `nix` 77G, `persist` ~40G *(just cleaned from 600G+ by de-duplication — `zfs list` still shows the stale 646G figure until old snapshots pinning those blocks are destroyed)*, `cache` 433G (mostly the Steam library), `log` 55M | pool 174G used, 268G available |
| Free space on disks | **~1.7 MiB per disk** — partition tables are fully allocated, no room to carve a new layout in place |

Live pool usage, once the stale persist snapshots are dropped, is roughly
77 + 40 + 433 ≈ **550 GiB**, which fits comfortably on *one* of the two
~915 GiB disks. ZFS's device-removal feature can evacuate one disk's vdev
onto the other while the pool stays online — this is what makes phase 1
possible with no external backup drive at all.

## Target architecture

### Phase 1 (this round)

```
                 ┌─────────────────────────────────────────┐
   disk0         │ ESP (1G, vfat, /boot)                    │
   (new layout)  ├───────────────────────────────────────────
                 │ LUKS "crypt0"  ← interactive passphrase  │
                 │   └── LVM vg0                            │
                 │         ├── lv "swap"  (raw, resumeDevice)
                 │         └── lv "root"  (BTRFS)           │
                 │               ├── @nix     → /nix        │
                 │               ├── @persist → /persist    │
                 │               └── @log     → /var/log    │
                 └─────────────────────────────────────────┘

   disk1         ┌─────────────────────────────────────────┐
   (unchanged)   │ ZFS `pool0`, native encryption           │
                 │   └── dataset `cache` → /cache            │
                 │  (unlock mechanism/prompt unchanged from  │
                 │   today; not touched by this migration)   │
                 └─────────────────────────────────────────┘
```

`disk1` keeps its existing ZFS pool and encryption exactly as it is today.
Only the `cache` dataset is needed going forward — the `nix`/`persist`/`log`
datasets on `pool0` become stale once their data is copied over to `disk0`,
and should be destroyed afterward to reclaim space on `disk1` for `/cache`
to grow into (see runbook step 8).

Because `disk1`'s unlock mechanism isn't touched in phase 1, boot may still
involve two separate unlock prompts (LUKS `crypt0` + whatever ZFS's existing
prompt/keyfile behavior is) until phase 2 unifies them.

### Phase 2 (future, only when more space is needed)

```
   disk1         ┌─────────────────────────────────────────┐
   (phase 2)     │ LUKS "crypt1"  ← unlocked automatically  │
                 │   via keyfile at /keys/crypt1.key        │
                 │   (only reachable once disk0 is open)    │
                 │         └── BTRFS → /cache                │
                 └─────────────────────────────────────────┘
```

This reuses the same shape originally designed for this migration: `disk0`
gains a `@keys` BTRFS subvolume holding a keyfile for `crypt1`, and `crypt1`
auto-unlocks via that keyfile so there's still only one interactive prompt.
Full mechanism below, kept for when phase 2 is actually scheduled.

### Getting to exactly one password prompt (phase 2)

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

### RAID0 vs. separate filesystems per disk

Relevant when phase 2 happens, since your Steam library (`/cache`, ~433G) is
the main occupant of `disk1` and could live on either topology.

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

Sized for the *planned* 64 GiB RAM upgrade, not the current 32 GiB, per your
ask → **72 GiB swap LV** now. This "wastes" ~40G of disk relative to today's
actual need, but avoids a second disruptive resize later; if you'd rather
not commit the space yet, LVM makes shrinking this after the fact trivial
too — say the word and I'll size it for 32G+margin instead.

### Carrying over scrub / auto-snapshot / trim

During phase 1, this is a hybrid: `disk0`'s BTRFS subvolumes get the new
mechanisms below; `disk1`'s ZFS pool keeps its existing `zfs.autoScrub`/
`autoSnapshot`/`trim` config untouched until phase 2 retires it.

| ZFS today | BTRFS equivalent (disk0, from phase 1 onward) |
|---|---|
| `services.zfs.autoScrub` (monthly) | `services.btrfs.autoScrub` — built into nixpkgs, same shape (`enable`, `fileSystems`, `interval`) |
| `services.zfs.autoSnapshot` (frequent/hourly/daily/weekly) on `/persist` | `services.snapper` — `TIMELINE_CREATE`/`TIMELINE_LIMIT_HOURLY`/`_DAILY`/`_WEEKLY`/`_MONTHLY` per subvolume config, closest like-for-like to zfs-auto-snapshot's retention knobs |
| `services.zfs.trim` (weekly) | Already redundant: hallandren already sets `services.fstrim.enable = true`, which works at the block-device level regardless of filesystem. Just needs `settings.allowDiscards = true` on `crypt0` so TRIM passes through the encryption layer to the SSD. |

Example for the `filesystem` module (partially replaces `zfs.nix` — the ZFS
bits stay until phase 2, scoped down to just `pool0`/`disk1`):

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

### Phase 1 — `disk0` only

`hosts/hallandren/filesystem/disks.nix`:

```nix
let
  disk0 = "/dev/disk/by-id/nvme-Samsung_SSD_980_1TB_S649NU0W832448H";

  boot-size = "1G";
  swap-size = "72G"; # sized for a future 64G RAM upgrade, see fs.md
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

`hosts/hallandren/filesystem/btrfs.nix` (replaces `zfs.nix`'s root-pool
handling; keeps a scoped-down ZFS import for `disk1`'s `pool0`/`cache`):

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
  fileSystems."/cache".neededForBoot = true;
  fileSystems."/persist".neededForBoot = true;
  fileSystems."/var/log".neededForBoot = true;

  # disk1 stays on ZFS until phase 2 — keep pool0 importable
  boot.zfs.extraPools = [ "pool0" ];

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

Remove the ZFS root-pool config (whatever `zfs.nix`/`zpool.nix` did for the
old striped `pool0` root usage), but **keep** `boot.zfs.*`/
`supportedFilesystems = [ "zfs" ]`/kernel-package pinning — `disk1` still
needs them until phase 2.

### Phase 2 (future) — converting `disk1`

Only apply this once phase 2 is actually scheduled. Adds a `@keys`
subvolume to `disk0`'s `root` LV, and a new `disk1` entry:

```nix
disko.devices.disk.disk1 = {
  type = "disk";
  device = "/dev/disk/by-id/nvme-eui.0025385501b07a5d";
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
```

Add to `disk0`'s `root` subvolumes:

```nix
"/keys" = {
  mountpoint = "/keys";
  mountOptions = [ "noatime" ];
};
```

And add `fileSystems."/keys".neededForBoot = true;` to `btrfs.nix`, remove
the `boot.zfs.extraPools` line and (once nothing else needs it) ZFS support
entirely.

## Migration runbook (phase 1)

### 0. Before touching anything

- Commit the new `disko.nix`/`btrfs.nix` modules on a branch; `nix flake check`
  and `nixos-rebuild build --flake .#hallandren` (build only, don't switch)
  to confirm they evaluate.
- `git log`/`git status` clean; note down the currently-booted generation
  in case you need to roll back the *config* (this doesn't help once `disk0`
  is wiped — that's what the ZFS evacuation is for).
- Double check `/dev/disk/by-id/...` names with
  `ls -l /dev/disk/by-id/ | grep nvme` — don't rely on `/dev/nvme0n1` device
  ordering.

### 1. Reclaim stale snapshots (still normally booted)

```
zfs list -t snapshot -r pool0/root/persist
zfs destroy -r pool0/root/persist@<oldest>%<newest>   # or destroy individually
zfs list pool0/root/persist    # confirm ~40G, not ~640G
```

### 2. Evacuate `disk0`'s vdev onto `disk1` (still normally booted)

This is a supported *online* ZFS operation, safe to run on the live system
(it'll take a while and add some I/O load; the pool stays usable
throughout):

```
zpool status pool0   # note the exact vdev name for disk0's partition
zpool remove pool0 <disk0-vdev-name>
zpool status pool0   # watch "remove: ... in progress" to completion
```

Confirm afterwards that `zpool status` shows only `disk1`'s device and that
`df`/`zfs list` usage (~550G) comfortably fits `disk1`'s ~915G capacity.

### 3. Kexec (or boot USB) into a temporary environment

The running system is about to go away for a while — this is a maintenance
window:

```
nix run github:nix-community/nixos-anywhere -- \
  --kexec-only --flake /persist/nixos#hallandren --target-host nixos@localhost
```

### 4. Import the (now single-vdev) pool to pull data from

```
zpool import -f -R /mnt-old pool0
```

(Not read-only — `disk1` keeps running this pool going forward, so a normal
import is fine and is what phase-1's `boot.zfs.extraPools` expects at
runtime too.)

### 5. Run `disko` for `disk0`'s new layout

Only `disk0` is in `disko.devices.disk` for phase 1, so `disk1`/`pool0` is
untouched:

```
nix run github:nix-community/disko -- --mode disko --flake /persist/nixos#hallandren
```

This prompts once for `crypt0`'s passphrase and leaves the new layout
mounted at `/mnt`.

### 6. Pull the data from `disk1`

```
rsync -aHAX --info=progress2 /mnt-old/persist/ /mnt/persist/
rsync -aHAX --info=progress2 /mnt-old/var/log/ /mnt/var/log/
```

(`/nix` doesn't need copying — `nixos-install` populates it. `/cache`
doesn't need copying either — it stays live on `pool0`/`disk1`.)

### 7. Install

```
nixos-install --root /mnt --flake /persist/nixos#hallandren
```

### 8. Reclaim space on `disk1`

Now that `nix`/`persist`/`log` live on `disk0`, destroy their now-stale
datasets on `pool0` so `disk1`'s capacity is fully available to `cache`:

```
zfs destroy -r pool0/root/nix
zfs destroy -r pool0/root/persist
zfs destroy -r pool0/root/log
zfs list pool0   # confirm only cache remains
```

### 9. Reboot and verify

- Boots with `crypt0`'s passphrase prompt (plus whatever `disk1`'s existing
  ZFS unlock behavior already is — unchanged).
- `swapon --show` lists the new LV.
- `systemctl status snapper-timeline.timer` and the btrfs scrub timer are
  active for `disk0`; ZFS's existing scrub/snapshot timers still cover
  `pool0`/`disk1`.
- `zramctl`/`free -h` look sane.
- `df -h /nix /persist /var/log /cache` — first three on the new BTRFS LVs,
  `/cache` still on `pool0`.

### 10. Test hibernation

`systemctl hibernate`, confirm it resumes cleanly. Do this *before*
considering phase 1 done — it's the main feature being migrated for, and
the failure mode (corrupt resume, black screen) is much easier to debug
right after the change than after weeks of drift.

## Phase 2 (future, when more space is needed)

Outline only — flesh out into a full runbook when actually scheduled:

1. Stage `/cache`'s data somewhere temporary (disk0 has headroom once
   phase 1 lands, or use external storage).
2. Apply the phase-2 disko additions above (`@keys` on disk0, `crypt1` +
   BTRFS on disk1).
3. Generate `/mnt/keys/crypt1.key`, format/open `crypt1` with it, `mkfs.btrfs`.
4. Restore `/cache`'s data onto the new `crypt1` BTRFS volume.
5. `zpool export pool0` (or destroy it, once confirmed no longer needed) —
   ZFS can now be fully removed from the flake.
6. Reboot, verify: still one password prompt (crypt0 interactive, crypt1
   auto-unlocks via keyfile), `lsblk` shows both LUKS containers open,
   hibernation still resumes, `cryptsetup luksDump ... | grep Discards`
   confirms `allowDiscards` on both containers, `fstrim -av` runs clean.
7. Remove the `zfs` flake input if nothing else in the repo uses it.
8. `networking.hostId` can stay — it's harmless once ZFS is gone, just
   vestigial (it was originally there to guard against double pool imports).

## Risk notes

- Step 2 of the runbook (`zpool remove`) is the only "online"
  destructive-adjacent step — it's a well-supported ZFS feature but leaves a
  permanent indirect-mapping table on the pool. That's fine here since
  `pool0` keeps living on `disk1` long-term as a normal (non-removed-from)
  pool from that point on.
- Everything from "run `disko`" (step 5) onward is a point of no return for
  `disk0`'s old partition table. Don't proceed past that point without
  confirming step 2's `zpool remove` fully completed and `pool0` is healthy
  on `disk1` alone.
- Test hibernation before considering phase 1 done, same reasoning as
  always: easier to debug right after the change than after weeks of drift.
