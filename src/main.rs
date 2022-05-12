use std::any::TypeId;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::sync::Weak;

use bevy::prelude::*;
use bevy::reflect::*;
use mlua::prelude::*;
use mlua::*;

#[allow(unused_macros)]
macro_rules! impl_lua_newtype {
    (
        $ty:ident
        $(fields {
            $($field:tt),* $(,)?
        })?
        $(methods {
            $($method:tt),* $(,)?
        })?
        $(Debug($target:ident))?
        $(Component)?
    ) => {
        paste::paste! {
            struct [<Lua $ty>] ($ty);

            impl UserData for [<Lua $ty>] {
                $(fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
                    $(
                        fields.add_field_method_get(stringify!($field), |_, this| Ok(this.0.$field));
                        fields.add_field_method_set(stringify!($field), |_, this, val| Ok(this.0.$field = val));
                    )*
                })?
    
                fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
                    $($(
                        $($method)*
                    )*)?

                    $(impl_lua_newtype!(@Debug $target methods);)?
                }
            }

            // impl<'lua> ::mlua::FromLua<'lua> for [<Lua $ty>] {
            //     fn from_lua(lua_value: ::mlua::Value, _: &::mlua::Lua) -> ::mlua::Result<Self> {
            //         match lua_value {
            //             Value::UserData(userdata) => userdata.take::<Self>(),
            //             _ => Err(Error::FromLuaConversionError { from: lua_value.type_name(), to: stringify!([<Lua $ty>]), message: None })
            //         }
            //     }
            // }

            impl LuaNewtype for $ty {
                type Newtype = [<Lua $ty>];
            
                fn wrap(self) -> Self::Newtype {
                    [<Lua $ty>](self)
                }
            
                fn unwrap(newtype: Self::Newtype) -> Self {
                    newtype.0
                }
            }
        }
    };

    (@Debug newtype $methods:ident) => {
        $methods.add_meta_method(MetaMethod::ToString, |lua, this, ()| {
            format!("{:?}", this).to_lua(lua)
        })
    };

    (@Debug original $methods:ident) => {
        $methods.add_meta_method(MetaMethod::ToString, |lua, this, ()| {
            format!("{:?}", this.0).to_lua(lua)
        })
    }
}

struct BevyLua(Mutex<Lua>);

impl Deref for BevyLua {
    type Target = Mutex<Lua>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BevyLua {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

fn main() {
    App::new()
    .add_plugins(DefaultPlugins)
    .insert_resource(BevyLua(Mutex::new(Lua::new())))
    .add_startup_system(setup)
    .add_system(print)
    .add_system(lua_host.exclusive_system().at_end())
    .run();
}

fn setup(mut commands: Commands) {
    commands.spawn().insert(Transform::default());
}

fn print(q: Query<&Transform>) {
    for tf in q.iter() {
        println!("Transform value: {tf:?}");
    }
}

// #[derive(Clone)]
// struct LuaComponentRef {
//     entity: Entity,
//     meta: ReflectComponent,
//     ty_name: &'static str,
//     world: LuaWorldRef,
// }

// impl std::fmt::Debug for LuaComponentRef {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.write_fmt(format_args!("&{:?}.{}", self.entity, self.ty_name))
//     }
// }

// #[derive(Clone)]
// struct LuaComponentPathRef {
//     comp: LuaComponentRef,
//     path: std::string::String,
// }

// impl std::fmt::Debug for LuaComponentPathRef {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.write_fmt(format_args!("&{:?}.{}.{}", self.comp.entity, self.comp.ty_name, self.path))
//     }
// }

// struct LuaComponentValueRef<T: Reflect> {
//     path: LuaComponentPathRef,
//     value: T
// }

// impl<T: Reflect> LuaComponentValueRef<T> {
    
// }

// impl UserData for LuaComponentRef {
//     fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
//         fields.add_meta_field_with(
//             MetaMethod::Index, 
//             |lua| lua.create_function(
//                 |lua, (this, key): (AnyUserData, String)| { 
//                     let path = key.to_str().unwrap();

//                     // Nonexhaustive direct value list: i8-size cast to Integer, f32-64 cast to Number, all Vec combinations as custom userdata, strings
//                     let comp = this.borrow::<LuaComponentRef>().unwrap();

//                     let world = comp.world.lock();
//                     let world = world.read().unwrap();

//                     match comp.meta.reflect_component(&world, comp.entity) {
//                         Some(dynam) => match dynam.reflect_ref() {
//                             ReflectRef::Struct(dynam) => match dynam.field(path) {
//                                 Some(field) => match field.reflect_ref() {
//                                     // The value is a recognized, manipulable value; convert directly to it and copy rather than reference
//                                     ReflectRef::Value(value) => {
//                                         if let Some(value) = value.downcast_ref::<f32>() {
//                                             Ok(value.to_lua(lua).unwrap())
//                                         } else if let Some(value) = value.downcast_ref::<f64>() {
//                                             Ok(value.to_lua(lua).unwrap())
//                                         } else if let Some(value) = value.downcast_ref::<Vec3>() {
//                                             Ok(LuaVec3(*value).to_lua(lua).unwrap())
//                                         } else {
//                                             Ok(LuaComponentPathRef { comp: comp.clone(), path: path.to_string() }.to_lua(lua).unwrap())
//                                         }
//                                     },
//                                     // The value at the new path is not a recognized value but exists
//                                     _ => Ok(LuaComponentPathRef { comp: comp.clone(), path: path.to_string() }.to_lua(lua).unwrap())
//                                 },
//                                 // The path is invalid
//                                 None => Ok(Nil)
//                             }
//                             // The component is... weird ig
//                             _ => Ok(LuaComponentPathRef { comp: comp.clone(), path: path.to_string() }.to_lua(lua).unwrap())
//                         },
//                         // No component on this entity
//                         None => Ok(Nil),
//                     }
//                 }
//             )
//         );
//     }
    
//     fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
//         methods.add_meta_method(MetaMethod::ToString, |lua, this, ()| {
//             format!("{this:?}").to_lua(lua)
//         });
//     }
// }

// impl UserData for LuaComponentPathRef {
//     fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
//         fields.add_meta_field_with(
//             MetaMethod::Index, 
//             |lua| lua.create_function(
//                 |lua, (this, key): (AnyUserData, String)| { 
//                     let comp_ref = this.borrow::<LuaComponentPathRef>().unwrap();
//                     let comp = &comp_ref.comp;
//                     let path = &comp_ref.path;

//                     let path = path.to_owned() + "." + key.to_str().unwrap();

//                     let world = comp.world.lock();
//                     let world = world.read().unwrap();

//                     // Nonexhaustive direct value list: i8-size cast to Integer, f32-64 cast to Number, all Vec combinations as custom userdata, strings
//                     match comp.meta.reflect_component(&world, comp.entity) {
//                         Some(dynam) => match dynam.path(&path) {
//                             Ok(value) => match value.reflect_ref() {
//                                 ReflectRef::Value(value) => {
//                                     if let Some(value) = value.downcast_ref::<f32>() {
//                                         Ok(value.to_lua(lua).unwrap())
//                                     } else if let Some(value) = value.downcast_ref::<f64>() {
//                                         Ok(value.to_lua(lua).unwrap())
//                                     } else if let Some(value) = value.downcast_ref::<Vec3>() {
//                                         Ok(LuaVec3(*value).to_lua(lua).unwrap())
//                                     } else {
//                                         // The path value is a value but not a recognized one
//                                         Ok(LuaComponentPathRef { comp: comp.clone(), path }.to_lua(lua).unwrap())
//                                     }
//                                 },
//                                 // The value at the new path is not a recognized value but exists and is valid
//                                 _ => Ok(LuaComponentPathRef { comp: comp.clone(), path }.to_lua(lua).unwrap())
//                             },
//                             // New path is invalid
//                             _ => Ok(Nil)
//                         },
//                         // No component on this entity
//                         None => Ok(Nil),
//                     }
//                 }
//             )
//         );

//         fields.add_meta_field_with(
//             MetaMethod::NewIndex, 
//             |lua| lua.create_function(
//                 |lua, (this, key, rvalue): (AnyUserData, String, Value)| { 
//                     let comp_ref: &mut LuaComponentPathRef = &mut this.borrow_mut::<LuaComponentPathRef>().unwrap();

//                     println!("NewIndex called on reference {comp_ref:?} with key {key:?}!");

//                     let path = comp_ref.path.to_owned() + "." + key.to_str().unwrap();

//                     let world = comp_ref.comp.world.lock();
//                     let mut world = world.write().unwrap();

//                     // Nonexhaustive direct value list: i8-size cast to Integer, f32-64 cast to Number, all Vec combinations as custom userdata, strings
//                     match comp_ref.comp.meta.reflect_component_mut(&mut world, comp_ref.comp.entity) {
//                         Some(mut dynam) => match dynam.path_mut(&path) {
//                             Ok(lvalue) => match lvalue.reflect_mut() {
//                                 ReflectMut::Value(lvalue) => {
//                                     if let Some(value) = lvalue.downcast_mut::<f32>() {
//                                         *value = lua.coerce_number(rvalue).unwrap().unwrap() as f32;
//                                         Ok(Nil)
//                                     } else if let Some(value) = lvalue.downcast_mut::<f64>() {
//                                         *value = lua.coerce_number(rvalue).unwrap().unwrap() as f64;
//                                         Ok(Nil)
//                                     } else if let Some(value) = lvalue.downcast_mut::<Vec3>() {
//                                         *value = match rvalue {
//                                             Value::UserData(any_data) => any_data.borrow::<LuaVec3>().unwrap().0,
//                                             _ => return Err(Error::RuntimeError("Attempted to assign invalid type to lvalue of type Vec3".to_string()))
//                                         };
//                                         Ok(Nil)
//                                     } else {
//                                         // The path value is a value but not a recognized one;
//                                         // Have to consume the rvalue and apply it to the lvalue
//                                         let LuaComponentPathRef {
//                                             comp: LuaComponentRef {
//                                                 entity,
//                                                 meta,
//                                                 ty_name: _,
//                                                 world
//                                             },
//                                             path: _
//                                         } = match rvalue {
//                                             Value::UserData(rvalue) => rvalue.take::<LuaComponentPathRef>().unwrap(),
//                                             // The value is not a path reference; don't know what to do here, so don't
//                                             _ => return Err(Error::RuntimeError(format!("Attempted to assign invalid type to lvalue of type {}", comp_ref.comp.ty_name)))
//                                         };

//                                         let world = world.lock();
//                                         let world = world.read().unwrap();

//                                         let rvalue = match meta.reflect_component(&world, entity) {
//                                             Some(value) => value.clone_value(),
//                                             None => return Err(Error::RuntimeError(format!("Attempted to assign invalid type to lvalue of type {}", comp_ref.comp.ty_name)))
//                                         };

//                                         lvalue.set(rvalue).unwrap();

//                                         Ok(Nil)
//                                     }
//                                 },
//                                 // The value at the new path is not a recognized value but exists
//                                 _ => {
//                                     let LuaComponentPathRef {
//                                         comp: LuaComponentRef {
//                                             entity,
//                                             meta,
//                                             ty_name: _,
//                                             world
//                                         },
//                                         path: _
//                                     } = match rvalue {
//                                         Value::UserData(rvalue) => rvalue.take::<LuaComponentPathRef>().unwrap(),
//                                         // The value is not a path reference; don't know what to do here, so don't
//                                         _ => return Err(Error::RuntimeError(format!("Attempted to assign invalid type to lvalue of type {}", comp_ref.comp.ty_name)))
//                                     };

//                                     let world = world.lock();
//                                     let world = world.read().unwrap();

//                                     let rvalue = match meta.reflect_component(&world, entity) {
//                                         Some(value) => value.clone_value(),
//                                         None => return Err(Error::RuntimeError(format!("Attempted to assign invalid type to lvalue of type {}", comp_ref.comp.ty_name)))
//                                     };

//                                     lvalue.set(rvalue).unwrap();

//                                     Ok(Nil)
//                                 }
//                             },
//                             // New path is invalid
//                             _ => return Err(Error::RuntimeError(format!("Attempted to assign invalid type to lvalue of type {}", comp_ref.comp.ty_name)))
//                         },
//                         // No component on this entity
//                         None => return Err(Error::RuntimeError(format!("Attempted to assign invalid type to lvalue of type {}", comp_ref.comp.ty_name)))
//                     }
//                 }
//             )
//         );
//     }

//     fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
//         methods.add_meta_method(MetaMethod::ToString, |lua, this, ()| {
//             format!("{this:?}").to_lua(lua)
//         });
//     }
// }

struct LuaEntity {
    entity: Entity,
    world: LuaWorldRef
}

impl UserData for LuaEntity {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("despawn", |_, this, _: ()| {
            Ok(this.world.lock().write().unwrap().despawn(this.entity))
        });
        
        methods.add_method_mut("get", |lua, this, comp_name: String| {
            let world = this.world.lock();
            let world = world.read().unwrap();

            let registry: &TypeRegistry = world.get_resource().unwrap();

            let comp = (|| {
                let registry = registry.read();

                let reg = 
                    comp_name.to_str()
                    .map(|s| registry.get_with_short_name(s).or(registry.get_with_name(s)))
                    .ok()??;

                let comp = reg.data::<ReflectComponent>()?.to_owned();

                let comp_id = reg.type_id();
                let comp_name = reg.name();

                let comp = LuaCompRef { 
                    world: this.world.clone(),
                    entity: this.entity,
                    comp,
                    comp_id,
                    comp_name,
                    path: None
                };
                
                Some(comp)
            })();

            comp.map(|comp| comp.to_lua(lua)).or(Some(Ok(Nil))).unwrap()
        });
    }
}

trait LuaNewtype {
    type Newtype;

    fn wrap(self) -> Self::Newtype;

    fn unwrap(newtype: Self::Newtype) -> Self;
}

impl_lua_newtype! {
    Vec3
    fields {
        x,
        y,
        z,
    }
    Debug(original)
}

#[derive(Clone)]
struct LuaWorldRef(Weak<RwLock<World>>);

impl LuaWorldRef {
    fn lock(&self) -> Arc<RwLock<World>> {
        self.upgrade().unwrap()
    }
}

impl Deref for LuaWorldRef {
    type Target = Weak<RwLock<World>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LuaWorldRef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone)]
struct LuaCompRef {
    world: LuaWorldRef,
    entity: Entity,
    comp: ReflectComponent,
    comp_id: TypeId,
    comp_name: &'static str,
    path: Option<std::string::String>,
}

impl LuaCompRef {
    fn eval<'m>(&mut self) -> Result<LuaValueRef> {
        let world = self.world.lock();
        let world = world.read().unwrap();

        let comp_ref = 
            self.comp.reflect_component(&world, self.entity)
            .ok_or(Error::RuntimeError("Component does not exist on this entity".to_string()))?;

        if let Some(path) = &self.path {
            let mut path = &path[..];
            let mut path_extra = &path[0..0];
    
            while let Err(_) = comp_ref.path(&path) {
                if let Some(dot) = path.rfind('.') {
                    path = &path[..dot];
                    path_extra = &path[dot..];
                } else {
                    break;
                }
            }
    
            let latest_valid = match comp_ref.path(&path) {
                Ok(lvalue) => Ok(lvalue),
                Err(_) => Err(Error::RuntimeError(format!("The path {:?} is invalid", self)))
            }?.reflect_ref();
    
            if path_extra.len() == 0 {
                let value_ty = match latest_valid {
                    ReflectRef::Struct(_) => LuaValueRefType::Struct,
                    ReflectRef::TupleStruct(_) => LuaValueRefType::Struct,
                    ReflectRef::Tuple(_) => LuaValueRefType::Struct,
                    ReflectRef::Value(val) => {
                        if val.is::<f32>() {
                            LuaValueRefType::F32
                        } else if val.is::<f64>() {
                            LuaValueRefType::F64
                        } else if val.is::<Vec3>() {
                            LuaValueRefType::Vec3
                        } else if val.is::<IVec3>() {
                            LuaValueRefType::IVec3
                        } else if val.is::<UVec3>() {
                            LuaValueRefType::UVec3
                        } else {
                            return Err(Error::RuntimeError(format!("The path {:?} points to an invalid type", self)))
                        }
                    },
                    _ => return Err(Error::RuntimeError(format!("The path {:?} points to an invalid type", self)))
                };
    
                return Ok(LuaValueRef {
                    reference: ReferenceBase::Component(self.clone()),
                    path: None,
                    value_ty,
                });
            }
    
            match latest_valid {
                ReflectRef::Value(value) => {
                    if value.is::<Vec3>() {
                        match path_extra {
                            ".x" | ".y" | ".z" => Ok(LuaValueRef {
                                reference: ReferenceBase::Component(LuaCompRef {
                                    path: Some(path.to_string()),
                                    ..self.clone()
                                }),
                                path: Some(path_extra.to_string()),
                                value_ty: LuaValueRefType::F32,
                            })
                        }
                    } else if value.is::<IVec3>() {
                        match path_extra {
                            ".x" | ".y" | ".z" => Ok(LuaValueRef {
                                reference: ReferenceBase::Component(LuaCompRef {
                                    path: Some(path.to_string()),
                                    ..self.clone()
                                }),
                                path: Some(path_extra.to_string()),
                                value_ty: LuaValueRefType::I32,
                            })
                        }
                    } else if value.is::<UVec3>() {
                        match path_extra {
                            ".x" | ".y" | ".z" => Ok(LuaValueRef {
                                reference: ReferenceBase::Component(LuaCompRef {
                                    path: Some(path.to_string()),
                                    ..self.clone()
                                }),
                                path: Some(path_extra.to_string()),
                                value_ty: LuaValueRefType::U32,
                            })
                        }
                    } else {
                        Err(Error::RuntimeError(format!("The path {:?} points to an invalid type", self)))
                    }
                },
                _ => Err(Error::RuntimeError(format!("The path {:?} is invalid", self)))
            }
        } else {
            Ok(LuaValueRef {
                reference: ReferenceBase::Component(self.clone()),
                path: None,
                value_ty: LuaValueRefType::Struct,
            })
        }
    }
}

impl std::fmt::Debug for LuaCompRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path = match &self.path {
            Some(path) => format!(".{}", path),
            None => "".to_string(),
        };
        f.write_fmt(format_args!("&{:?}.{}{}", self.entity, self.comp_name, path))
    }
}

fn userdata<T>(value: Value) -> Result<AnyUserData> {
    match value {
        Value::UserData(userdata) => Ok(userdata),
        _ => Err(Error::FromLuaConversionError { from: value.type_name(), to: std::any::type_name::<T>(), message: None })
    }
}

fn reflect_mut_type_name<'r: 'm, 'm>(refl: &'r ReflectMut<'m>) -> &'m str {
    match refl {
        ReflectMut::Struct(refl) => refl.type_name(),
        ReflectMut::TupleStruct(refl) => refl.type_name(),
        ReflectMut::Tuple(refl) => refl.type_name(),
        ReflectMut::List(refl) => refl.type_name(),
        ReflectMut::Map(refl) => refl.type_name(),
        ReflectMut::Value(refl) => refl.type_name(),
    }
}

impl UserData for LuaCompRef {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field_with(MetaMethod::Index, |lua| {
            lua.create_function(|_lua, (base, key): (Value, String)| {
                let any = userdata::<LuaCompRef>(base)?;
                let base = any.borrow::<LuaCompRef>()?;

                let path = match &base.path {
                    Some(base_path) => Some(base_path.to_owned() + "." + key.to_str()?),
                    None => Some(key.to_str()?.to_string()),
                };

                Ok(LuaCompRef {
                    world: base.world.clone(),
                    entity: base.entity,
                    comp: base.comp.clone(),
                    comp_id: base.comp_id,
                    comp_name: base.comp_name,
                    path,
                })
            })
        });

        fields.add_meta_field_with(MetaMethod::NewIndex, |lua| {
            lua.create_function(|_lua, (base, key, value): (Value, String, Value)| {
                let any = userdata::<LuaCompRef>(base)?;
                let base = any.borrow_mut::<LuaCompRef>()?;

                let world = base.world.lock();
                let world = world.read().unwrap();

                let mut comp_ref = 
                    // SAFETY: This is called from within an exclusive system running the lua,
                    // and we check later to ensure we don't get a mutable + immutable reference to a component
                    unsafe { base.comp.reflect_component_unchecked_mut(&world, base.entity) }
                    .ok_or(Error::RuntimeError("Component does not exist on this entity".to_string()))?;

                let rvalue: Box<dyn Reflect> = match value {
                    Nil => Box::new(()),
                    Value::Boolean(bool) => Box::new(bool),
                    Value::LightUserData(_light) => todo!(),
                    Value::Integer(int) => Box::new(int),
                    Value::Number(float) => Box::new(float),
                    Value::String(string) => Box::new(string.to_str()?.to_string()),
                    Value::Table(_table) => todo!(),
                    Value::Function(_func) => todo!(),
                    Value::Thread(_threa) => todo!(),
                    Value::UserData(userdata) => {
                        let rvalue_ref = userdata.borrow::<LuaCompRef>()?;

                        // SAFETY: This check is necessary to prevent breaking reference rules
                        if rvalue_ref.entity == base.entity && rvalue_ref.comp_id == base.comp_id {
                            // Assigning a component to itself is disallowed because it would be difficult to avoid
                            // a simultaneous mutable/immutable reference of the same data
                            return Err(Error::RuntimeError("Assigning a component reference to itself is disallowed. Try cloning the value first.".to_string()));
                        }

                        let comp = 
                            rvalue_ref.comp
                            .reflect_component(&world, rvalue_ref.entity)
                            .ok_or(Error::RuntimeError("Rvalue component does not exist on entity".to_string()))?;

                        if let Some(path) = &rvalue_ref.path {
                            comp.path(path).map_err(|_| Error::RuntimeError("Path to rvalue field is invalid".to_string()))?.clone_value()
                        } else {
                            comp.clone_value()
                        }
                    },
                    Value::Error(err) => return Err(Error::RuntimeError(format!("Attempted to assign error to reference. Inner error: {:?}", err))),
                };

                match &mut lvalue {
                    ReflectMut::Struct(lvalue) => lvalue.set(rvalue),
                    ReflectMut::TupleStruct(lvalue) => lvalue.set(rvalue),
                    ReflectMut::Tuple(lvalue) => lvalue.set(rvalue),
                    ReflectMut::List(lvalue) => lvalue.set(rvalue),
                    ReflectMut::Map(lvalue) => lvalue.set(rvalue),
                    ReflectMut::Value(lvalue) => {
                        if lvalue.is::<f32>() {
                            lvalue.set(Box::new(*rvalue.downcast::<Number>().unwrap() as f32))
                        } else if lvalue.is::<f64>() {
                            lvalue.set(Box::new(*rvalue.downcast::<Number>().unwrap() as f64))
                        } else {
                            lvalue.set(rvalue)
                        }
                    },
                }.map_err(|rvalue| Error::RuntimeError(format!("Failed to assign rvalue {} to lvalue {}", rvalue.type_name(), reflect_mut_type_name(&lvalue))))?;

                Ok(Nil)
            })
        });
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |lua, this, ()| {
            let (path_dot, path_str): (&str, &str) = match &this.path {
                Some(path) => (".", path),
                None => ("", "")
            };
            format!("&{:?}.{}{}{}", this.entity, this.comp_name, path_dot, path_str).to_lua(lua)
        });

        methods.add_method("clone", |lua, this, ()| {
            let world = this.world.lock();
            let world = world.read().unwrap();

            let mut value = 
                this.comp.reflect_component(&world, this.entity)
                .ok_or(Error::RuntimeError(format!("Component {} is not present on entity {:?}", this.comp_name, this.entity)))?;

            if let Some(path) = &this.path {
                value = value.path(path).map_err(|_| Error::RuntimeError(format!("Path {}.{} is invalid", this.comp_name, path)))?;
            }

            match value.reflect_ref() {
                ReflectRef::Struct(_) => todo!(),
                ReflectRef::TupleStruct(_) => todo!(),
                ReflectRef::Tuple(_) => todo!(),
                ReflectRef::List(_) => todo!(),
                ReflectRef::Map(_) => todo!(),
                ReflectRef::Value(val) => {
                    if val.is::<f32>() {
                        val.downcast_ref::<f32>().unwrap().to_lua(lua)
                    } else if val.is::<Vec3>() {

                    } else {
                        todo!()
                    }
                },
            }
        })
    }
}

enum ReferenceBase {
    Component(LuaCompRef),
    Owned(Arc<RwLock<dyn Reflect>>),
}

impl std::fmt::Debug for ReferenceBase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Component(comp_ref) => f.write_fmt(format_args!("{comp_ref:?}")),
        }
    }
}

enum LuaValueRefType {
    Struct,
    F32,
    F64,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    String,
    Vec2,
    IVec2,
    UVec2,
    Vec3,
    IVec3,
    UVec3,
    Vec4,
    IVec4,
    UVec4,
}

struct LuaValueRef {
    reference: ReferenceBase,
    path: Option<std::string::String>,
    value_ty: LuaValueRefType,
}

impl LuaValueRef {
    fn eval(&mut self) -> &mut dyn Reflect {
        match self.reference {
            ReferenceBase::Component(comp_ref) => comp_ref.eval(),
            ReferenceBase::Owned(val_ref) => {
                val_ref.
            }
        }
    }

    fn path(&self, path: &str) -> Result<&dyn Reflect> {
        match self.value_ty {
            LuaValueRefType::Struct => todo!(),
            LuaValueRefType::Vec2 => todo!(),
            LuaValueRefType::IVec2 => todo!(),
            LuaValueRefType::UVec2 => todo!(),
            LuaValueRefType::Vec3 => todo!(),
            LuaValueRefType::IVec3 => todo!(),
            LuaValueRefType::UVec3 => todo!(),
            LuaValueRefType::Vec4 => todo!(),
            LuaValueRefType::IVec4 => todo!(),
            LuaValueRefType::UVec4 => todo!(),
            _ => Err(Error::RuntimeError(format!("The path {:?} is invalid", self)))
        }
    }
}

impl std::fmt::Debug for LuaValueRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (dot, path): (&str, &str) = match &self.path {
            Some(path) => (".", path),
            None => ("", ""),
        };
        f.write_fmt(format_args!("{:?}{}{}", self.reference, dot, path))
    }
}

fn lua_host(world: &mut World) {
    let lua: BevyLua = world.remove_resource().unwrap();
    let time: Time = world.remove_resource().unwrap();

    // Notes on attempting to thread this:
    // Luas are basically impossible to pass around, but having multiple that each handle their own functionality could work.
    // For multiple mutable access to the world, either reimplement a *lot* of shit manually or wrap it in Arc<RwLock<<>>, which seems to 
    // potentially work quite well.

    let world_arc = Arc::new(RwLock::new(std::mem::take(world)));

    {
        let lua = lua.lock().expect("Failed to lock Lua mutex");

        let entities_to_modify: Vec<Entity>;
        {
            let mut world = world_arc.write().unwrap();
            entities_to_modify = world.query::<Entity>().iter(&world).collect();
        }

        lua.globals().set("deltaTime", time.delta_seconds()).unwrap();
        for entity in entities_to_modify {
            let world_ref = LuaWorldRef(Arc::downgrade(&world_arc));
            
            let lua_entt = LuaEntity { entity, world: world_ref };
            
            lua.globals().set("entity", lua_entt).unwrap();
            lua.load(r#"
                local tf = entity:get("Transform")
                print("Transform is: ", tf)
                print("Transform.translation is: ", tf.translation)
                local translation = tf.translation
                print("Transform.translation local is: ", tf.translation)
                print("Transform.translation.x is: ", tf.translation.x)
                local x = translation.x
                print("Transform.translation.x local is: ", x)
                local xval = x:clone()
                print("XVal is: ", xval)
            "#).exec().unwrap();
            lua.globals().set("entity", Nil).unwrap();
        }
    }

    *world = Arc::try_unwrap(world_arc).unwrap().into_inner().unwrap();

    world.insert_resource(lua);
    world.insert_resource(time);
}