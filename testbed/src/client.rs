use std::time::Instant;
use specs::prelude::*;
use crate::{
    net_marker::*,
    prot::*,
    ai::*,
    actor::*,
    input::*,
    consts::*,
    util::*,
};

mod state_buffer;

use self::state_buffer::StateBuffer;

pub fn new_client(server: LagNetwork<Input>, network: LagNetwork<WorldState>, is_ai: bool) -> Demo {
    let socket = Socket::new(network, server);

    let mut world = World::new();
    world.register::<Actor>();
    world.register::<NetMarker>();
    world.register::<StateBuffer>();

    world.add_resource(socket);
    world.add_resource(Reconciliation::new());
    world.add_resource(NetNode::new(0..2));

    if is_ai {
        world.add_resource::<Option<AI>>(Some(AI::new()));
        world.add_resource::<Option<Stick>>(None);
        //unimplemented!()
    } else {
        world.add_resource::<Option<AI>>(None);
        world.add_resource::<Option<Stick>>(Some(Stick::default()));
    }

    let dispatcher = DispatcherBuilder::new()
        .with(ProcessServerMessages, "ProcessServerMessages", &[])
        .with(ProcessInputs::new(), "ProcessInputs", &["ProcessServerMessages"])
        .with(InterpolateEntities, "InterpolateEntities", &["ProcessInputs"])
        .build();

    Demo::new(CLIENT_UPDATE_RATE, world, dispatcher)
}

// Get inputs and send them to the server.
// If enabled, do client-side prediction.
pub struct ProcessInputs {
    last_processed: Instant,
}

impl ProcessInputs {
    fn new() -> Self {
        Self { last_processed: Instant::now() }
    }
}

#[derive(SystemData)]
pub struct ProcessInputsData<'a> {
    me: ReadExpect<'a, Entity>,
    ai: Write<'a, Option<AI>>,
    stick: Write<'a, Option<Stick>>,
    reconciliation: WriteExpect<'a, Reconciliation>,
    socket: WriteExpect<'a, Socket<WorldState, Input>>,
    actors: WriteStorage<'a, Actor>,
}

impl<'a> System<'a> for ProcessInputs {
    type SystemData = ProcessInputsData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        // Compute delta time since last update.
        let dt = {
            let now = Instant::now();
            let last = std::mem::replace(&mut self.last_processed, now);
            duration_to_secs(now - last)
        };

        let me: Entity = *data.me;
        let actor = if let Some(actor) = data.actors.get_mut(me) {
            actor
        } else {
            return;
        };

        let ai = data.ai.as_mut()
            .and_then(|ai| ai.gen_stick(actor.position));

        let stick = data.stick.as_mut()
            .and_then(|s| s.take_updated()) // if nothing interesting happened.
            .or(ai);

        if let Some(stick) = stick {
            // Package player's input.
            let input = Input {
                press_time: dt,
                stick: stick.clone(),
                rotation: actor.rotation.angle(),
                sequence: data.reconciliation.sequence,
                entity_id: me.id() as usize,
            };

            data.reconciliation.sequence += 1;

            // Do client-side prediction.
            actor.apply_input(&input);
            // Send the input to the server.
            data.socket.send(input.clone());
            // Save self input for later reconciliation.
            data.reconciliation.save(input);
        }
    }
}

pub struct InterpolateEntities;

#[derive(SystemData)]
pub struct InterpolateEntitiesData<'a> {
    entities: Entities<'a>,
    me: ReadExpect<'a, Entity>,
    actors: WriteStorage<'a, Actor>,
    states: WriteStorage<'a, StateBuffer>,
}

impl<'a> System<'a> for InterpolateEntities {
    type SystemData = InterpolateEntitiesData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        // Compute render time.
        let render_time = Instant::now() -
            secs_to_duration(1.0 / SERVER_UPDATE_RATE);

        let me = *data.me;
        let actors = (&*data.entities, &mut data.actors, &mut data.states).join()
            // No point in interpolating self client's entity.
            //.filter_map(|(e, a, s)| if e == me { None } else { Some((a, s)) });
            .filter(|(e, _, _)| *e != me);

        for (e, actor, state) in actors {
            //actor.interpolate(render_time);
            if let Some((p, r)) = state.interpolate(render_time) {
                actor.position = p;
                actor.rotation = r;
            } else {
                //unimplemented!("extrapolation")
                println!("unimplemented extrapolation: me: {}, e: {}",
                         me.id(), e.id());
            }
        }
    }
}

// Process all messages from the server, i.e. world updates.
// If enabled, do server reconciliation.
pub struct ProcessServerMessages;

#[derive(SystemData)]
pub struct ProcessServerMessagesData<'a> {
    entities: Entities<'a>,
    reconciliation: WriteExpect<'a, Reconciliation>,
    socket: WriteExpect<'a, Socket<WorldState, Input>>,
    me: ReadExpect<'a, Entity>,
    actors: WriteStorage<'a, Actor>,
    states: WriteStorage<'a, StateBuffer>,
    lazy: Read<'a, LazyUpdate>,
}

impl<'a> System<'a> for ProcessServerMessages {
    type SystemData = ProcessServerMessagesData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        let now = Instant::now();
        let me = data.me.id() as usize;
        while let Some(message) = data.socket.recv() {
            let last_processed_input = message.last_processed_input;

            // World state is a list of entity states.
            for m in &message.states {
                // If self is the first time we see self entity,
                // create a local representation.
                let id = unsafe { std::mem::transmute((m.entity_id as u32, 1)) };
                let actor = data.actors.get_mut(id);
                let state = data.states.get_mut(id);

                let (actor, state) = if let (Some(actor), Some(state)) = (actor, state) {
                    (actor, state)
                } else {
                    data.lazy.create_entity(&data.entities)
                        .from_server(m.entity_id)
                        .with(Actor::spawn(m.position))
                        .with(StateBuffer::new())
                        .build();
                    continue;
                };

                if m.entity_id == me as u16 {
                    data.reconciliation.reconciliation(
                        actor,
                        m.position,
                        last_processed_input,
                    );
                } else {
                    // Received the position of an entity other than self client's.
                    // Add it to the position buffer.
                    state.push_state(now, m);
                }
            }
        }
    }
}
