def divergent_cells(snapshot, draft, pending):
    """A conflict must differ from both the draft and any uncertain sync ACK."""
    cells = snapshot['own']['cells'].copy()
    index = next(i for i, given in enumerate(snapshot['own']['givens']) if given == 0)
    command = ((pending or {}).get('envelope') or {}).get('command', {})
    forbidden = {cells[index], draft['cells'][index]['value']}
    if command.get('type') == 'sync':
        forbidden.add(command['cells'][index])
    cells[index] = next(value for value in range(1, snapshot['size'] + 1) if value not in forbidden)
    return cells, index
