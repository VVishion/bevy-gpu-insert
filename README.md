## Bevy Gpu Insert

Insert data from buffers on the Gpu to the `MainWorld`.

Take a look at the example: `cargo r --example generate`.

```rust
impl GpuInsert for GeneratedMesh {
    type Info = Handle<GeneratedMesh>;
    type Param = SResMut<Assets<GeneratedMesh>>;

    fn insert(
        data: &[u8],
        info: Self::Info,
        assets: &mut SystemParamItem<Self::Param>,
    ) -> Result<(), GpuInsertError> {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

        let data: Vec<_> = data
            .chunks_exact(4)
            .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
            .collect();

        ...

        let _ = assets.set(info, Self(mesh));

        Ok(())
    }
}
```

```rust
fn queue_gpu_inserts(
    mut gpu_insert_commands: ResMut<Vec<GpuInsertCommand<GeneratedMesh>>>,
) {
    gpu_insert_commands.push(GpuInsertCommand {
        buffer,
        bounds: 0..size,
        staging_buffer,
        staging_buffer_offset: 0,
        info: handle.clone_weak(),
    });
}
```
