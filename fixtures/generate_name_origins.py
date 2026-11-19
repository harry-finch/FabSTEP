"""Three independent cubes: empty, user-supplied and absent 3MF names."""
from generate_m3 import mesh_object, model, translate, write

write("m3_name_origins", model([
    mesh_object(1, name=""),
    mesh_object(2, name="Objet 1 · 1"),
    mesh_object(3).replace(' name="Cube"', ''),
], [(1, None), (2, translate(20, 0, 0)), (3, translate(40, 0, 0))]))

# The first byte of the combining accent would exceed the CAD name budget.
write("m3_name_grapheme", model([mesh_object(1, name="P" * 59 + "e\u0301 long")], [(1, None)]))
