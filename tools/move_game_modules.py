#!/usr/bin/env python3
"""One-shot #1455 move. Run --move on the old layout; --verify BASE proves the move."""
import argparse
from collections import Counter
import os
from pathlib import Path
import re
import subprocess

GAME = Path('crates/horizon-client/src/game')
DECL = re.compile(r'^(#\[cfg\(test\)\]\n)?(pub(?:\(crate\))? )?mod (\w+);\n\n?', re.M)
INCLUDE = 'include!(concat!(env!("OUT_DIR"), "/game_modules.rs"));\n'
SUPPORT = {'agent_navigation': 'agent', 'mod_inventory_tests': 'mods'}


def area(name):
    if name.startswith(('native_', 'mod_api_')) or name in {'mods', 'attachment_sync'}:
        return 'modding'
    if name.startswith(('lan', 'remote', 'net_')) or name in {'prediction', 'friends', 'server_links', 'social'}:
        return 'network'
    groups = {
        'mobs': 'allays beehives breezes cat_spawner copper_golems creaking creatures custom_spawners dragon enderman fangs finalize_spawn foxes ghasts golems llamas living_attributes mob_blocks mob_gear mob_index mob_looting mob_sounds mounts natural_spawning nautili ominous_item_spawner pandas piglins raids sheep_dye shoulders sniffers spawner sulfur_cubes trial_spawner villagers wandering_trader wither zombie_reinforcements zombie_villagers xp_orb_targeting',
        'entities': 'armor_stands boarding boat_authority_fixture boat_leads boat_passengers boat_rider_fixture boats cushions entity entity_sounds explosion falling_damage fireworks frames glowing leads lightning minecarts paintings passengers pick_entity pickups player_equip pvp text_displays tnt vehicle_teleport',
        'administration': 'functions game_tests jigsaw_blocks jigsaw_edit structure_blocks structure_edit structures tempban',
        'blocks': 'banners beacon beds block_edits block_entity_anims block_entity_ticks block_items brush campfire campfire_douse cartography chiseled_bookshelf composter decorated_pot debug_stick dispenser growth jukebox loom machines maps moving_pistons nether_portal piston_push potent_sulfur powder_snow pumpkin respawn_anchor sculk sculk_fixture shelf spill splash_water stations transient_blocks trial_visuals vault',
        'items': 'bundles combat consume creative_hotbar crossbow deflect durability enchant_effects fishing hand interactions interaction_shapes item_areas item_particles item_use mace potions recipe_book shield spear totem trading trident',
        'world': 'end initial_spawn integrated legacy_rules levels ordered_tick packs persist postprocess respawn_position saving server_lifecycle server_packs server_reload server_travel storage tick_profile tick_script world_loading',
    }
    if name.startswith(('command', 'test_')):
        return 'administration'
    for group, names in groups.items():
        if name in names.split():
            return group
    if name.startswith(('agent', 'blockctl', 'control', 'event', 'execution_', 'headless', 'input_', 'launch_', 'memory', 'no_resources', 'repro', 'saves_control', 'split', 'station_control', 'transactions', 'ui_control')):
        return 'runtime'
    return 'interface'


def plan(files, source):
    roots = {m[2] for m in DECL.findall(source)} - {'native_hooks'}
    moves = {}
    for path in files:
        if not path.startswith(f'{GAME}/'):
            continue
        rel = Path(path).relative_to(GAME)
        name = Path(rel.parts[0]).stem
        if name == 'native_hooks':
            continue
        owner = name if name in roots else SUPPORT.get(name, name.removesuffix('_tests'))
        assert owner in roots, (path, owner)
        tail = rel if name in roots else Path(owner) / rel
        has_children = any(p.startswith(f'{GAME}/{name}/') for p in files) or name in SUPPORT.values() or any(p == f'{GAME}/{name}_tests.rs' for p in files)
        if len(rel.parts) == 1 and name in roots and has_children:
            tail = Path(name) / 'mod.rs'
        moves[path] = str(GAME / area(owner) / tail)
    return moves


def rewrite(text, path, moves):
    if path == 'tools/move_game_modules.py':
        return text
    # Resolve source-relative Rust includes and #[path]s before replacing documented paths.
    if path.endswith('.rs'):
        def relative(match):
            old = os.path.normpath(str(Path(path).parent / match[1]))
            if old not in moves and not Path(old).is_file():
                return match[0]
            target = moves.get(old, old)
            new = os.path.relpath(target, Path(moves.get(path, path)).parent)
            return f'"{new}"'
        text = re.sub(r'"([^"\n]+\.rs)"', relative, text)
    short = {old.removeprefix('crates/horizon-client/src/'): new.removeprefix('crates/horizon-client/src/')
             for old, new in moves.items()}
    # Synthetic module-walker fixtures describe their own layout, not the real tree.
    if '/horizon-mod-api-gen/' not in path:
        pattern = '|'.join(re.escape(p) for p in sorted(short, key=len, reverse=True))
        if path.startswith('tests/') and path.endswith('.py'):
            def glob(match):
                name = match[2]
                if name.startswith('crates/'):
                    prefix = 'crates/horizon-client/src/'
                else:
                    prefix = ''
                target = short[name.removeprefix(prefix)]
                parts = target.split('/')
                parts[1] = '*'
                return f'next({match[1]}.glob({(prefix + "/".join(parts))!r}))'
            text = re.sub(r'(\b\w+)\s*/\s*[\'"]((?:crates/horizon-client/src/)?(?:' + pattern + r'))[\'"]', glob, text)
        text = re.sub(pattern, lambda m: short[m[0]], text)
    # The only real-tree read in this module; the other literals are synthetic fixtures.
    if path.endswith('/points.rs'):
        text = text.replace('read_to_string(root.join("crates/horizon-client/src/game/remote.rs"))',
                            'read_to_string(root.join("' + moves['crates/horizon-client/src/game/remote.rs'] + '"))')
    return text


def moved_source(source):
    first = True
    def remove(match):
        nonlocal first
        if match[3] == 'native_hooks':
            return match[0]
        assert bool(match[1]) == (match[3].endswith('_tests') and match[3] != 'game_tests'), match[0]
        replacement = INCLUDE if first else ''
        first = False
        return replacement
    return DECL.sub(remove, source)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--move', action='store_true')
    parser.add_argument('--verify', metavar='BASE')
    args = parser.parse_args()
    def git(*args):
        return subprocess.check_output(['git', *args]).decode()
    base_files = git('ls-tree', '-r', '--name-only', args.verify).splitlines() if args.verify else git('ls-files').splitlines()
    game = str(GAME) + '.rs'
    source = git('show', f'{args.verify}:{game}') if args.verify else Path(game).read_text()
    moves = plan(base_files, source)
    metadata = {str(GAME / (m[2] + '.rs')): '// game-module-visibility: ' + m[1].strip() + '\n'
                for m in DECL.findall(source) if m[1]}
    metadata[str(GAME / 'game_tests.rs')] = '// game-module-cfg: always\n'
    removed, added = Counter(), Counter()
    changed = []
    for old in base_files:
        path = Path(old)
        if not args.verify and (not path.is_file() or path.is_symlink()):
            continue
        data = subprocess.check_output(['git', 'show', f'{args.verify}:{old}']) if args.verify else path.read_bytes()
        if b'\0' in data:
            continue
        try:
            original = data.decode('utf-8')
        except UnicodeDecodeError:
            continue
        if not (old.startswith(('crates/', 'tests/', 'tools/', 'docs/', '.github/', 'scripts/')) or old.endswith('.md')):
            continue
        expected = moved_source(original) if old == game else original
        expected = metadata.get(old, '') + rewrite(expected, old, moves)
        new = moves.get(old, old)
        if args.verify:
            if old in moves or old == game:
                assert Path(new).read_text() == expected, f'undeclared edit: {old} -> {new}'
            if old in moves:
                # Normalize only the declared file-reference and visibility edits.
                removed.update(expected.splitlines())
                added.update(Path(new).read_text().splitlines())
        else:
            if old in moves:
                Path(new).parent.mkdir(parents=True, exist_ok=True)
                subprocess.run(['git', 'mv', old, new], check=True)
            if expected != original:
                Path(new).write_text(expected)
                changed.append(new)
    if args.verify:
        assert removed == added
        print(f'{len(moves)} files; {sum(removed.values())} normalized moved lines; removed/added multisets equal')
    else:
        for group in {area(Path(p).stem) for p in metadata} | {Path(p).parts[4] for p in moves.values()}:
            (GAME / group / '.game-area').write_text('Area roots are discovered; child folders belong to their module.\n')
        for directory in sorted(GAME.rglob('*'), key=lambda p: len(p.parts), reverse=True):
            if directory.is_dir() and not any(directory.iterdir()):
                directory.rmdir()
        print('\n'.join(changed))


if __name__ == '__main__':
    main()
