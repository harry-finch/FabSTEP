"""Three independent cubes: empty, user-supplied and absent 3MF names."""
from generate_m3 import mesh_object, model, translate, write

write("m3_name_origins", model([
    mesh_object(1, name=""),
    mesh_object(2, name="Objet 1 · 1"),
    mesh_object(3).replace(' name="Cube"', ''),
], [(1, None), (2, translate(20, 0, 0)), (3, translate(40, 0, 0))]))
