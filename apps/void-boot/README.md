# Void Boot

Void Boot es el centro gráfico de administración de arranque múltiple de Atdev Dev, integrado como una experiencia de escritorio de primera clase en Void Memingo.

**Void Boot está diseñado para usarse mediante interfaz gráfica.** No ofrece una interfaz de terminal (CLI) ni una interfaz de texto interactiva (TUI) como experiencia de usuario. Las operaciones se realizan desde ventanas, paneles, botones, selectores de archivos, indicadores de progreso y vistas previas.

## Experiencia gráfica

Desde el espacio de trabajo Void Boot de Void Memingo puedes:

- Explorar el catálogo de sistemas operativos y filtrar por categoría, familia y arquitectura.
- Consultar páginas oficiales, ediciones, formatos, arquitecturas y notas de compatibilidad.
- Elegir una carpeta para crear y administrar una biblioteca local de imágenes ISO.
- Inspeccionar ISO de forma no destructiva y consultar su tamaño, estructura ISO 9660, descriptor El Torito y SHA-256.
- Verificar una ISO con un manifiesto SHA-256 del editor mediante un selector gráfico de archivos. Se admiten los formatos GNU `SHA256SUMS` y BSD `SHA256 (filename) = digest`.
- Descargar y reanudar imágenes Linux desde enlaces HTTPS directos, con progreso, pausa y verificación SHA-256 opcional.
- Seleccionar catálogos de perfiles, validar su contenido y previsualizar el menú gráfico GRUB generado.
- Crear discos virtuales QCOW2 desechables y probarlos mediante QEMU cuando las dependencias estén instaladas.

El motor Rust es interno y compartido con Void Memingo; no constituye una interfaz de usuario independiente. La experiencia de usuario de Void Boot permanece en la aplicación gráfica.

## Catálogo y compatibilidad

El catálogo incluye 31 distribuciones Linux: Debian, Ubuntu, Fedora, Arch Linux, Linux Mint, Zorin OS, openSUSE, Pop!_OS, elementary OS, Kali Linux, EndeavourOS, Manjaro, MX Linux, NixOS, Void Linux, Rocky Linux, AlmaLinux, Alpine Linux, Mageia, CentOS Stream, Gentoo, Slackware, Lubuntu, Kubuntu, Xubuntu, Linux Lite, deepin, Solus, KDE neon, Oracle Linux y antiX.

También lista Windows 10 Home, Windows 10 Pro, Windows 10 Enterprise LTSC Evaluation, Windows 11 x64/ARM64, FreeBSD, OpenBSD, NetBSD y ReactOS. Los sistemas que no son Linux aparecen identificados como sistemas operativos externos, no como distribuciones Linux.

Las páginas oficiales se muestran separadas de la compatibilidad real de arranque. Un perfil es una receta de arranque, no una prueba de compatibilidad. Cada imagen requiere validación por versión, arquitectura y firmware. No se debe asumir que todos los ISO Linux admiten arranque loopback.

Windows, BSD y ReactOS son actualmente entradas de catálogo únicamente. No se ofrece compatibilidad multiboot o instalación no validada para esos sistemas. Windows 10 Home/Pro terminó su soporte general el 14 de octubre de 2025. La entrada LTSC apunta a medios de evaluación Enterprise de 90 días; el uso permanente requiere una licencia válida.

## Integridad y confianza

Void Boot calcula SHA-256 para cada ISO inspeccionada. La verificación gráfica contra un manifiesto compara el hash local con el valor publicado por el editor y rechaza entradas ausentes o contradictorias.

Una coincidencia de hash no autentica el manifiesto. Para comprobar autenticidad, el usuario debe obtener el manifiesto y su firma desde el editor oficial y verificar la firma con el procedimiento y las claves publicadas por ese proyecto. La verificación automática de firmas y la búsqueda automática de claves del editor aún no están implementadas.

## Laboratorio virtual y seguridad

Las pruebas de arranque se realizan con QEMU y discos virtuales desechables. Las imágenes seleccionadas se conectan en modo snapshot y la red de la máquina virtual permanece desactivada en el lanzador actual.

La inspección de ISO es de solo lectura. El inventario de dispositivos extraíbles no escribe en ellos. La escritura de medios físicos, el particionado, el formateo y la instalación de GRUB en unidades USB no están habilitados en esta etapa. El USB Ventoy existente debe permanecer intacto.

El menú gráfico previo al sistema operativo utiliza GRUB gfxterm y un tema de Void Boot. La creación del menú no significa que se haya instalado un cargador de arranque ni que un dispositivo físico sea arrancable.

## Estado de desarrollo

La aplicación gráfica ya integra catálogo, biblioteca ISO, descargas, verificación de integridad, vista previa de perfiles y laboratorio QEMU. Siguen pendientes la verificación automática de firmas de editor, perfiles validados para más versiones y arquitecturas, adaptadores de inventario para Windows/macOS, y flujos dedicados para medios de instalación de sistemas no Linux.

Consulta [Void Boot release gates](../../docs/VOID_BOOT_RELEASE_GATES.md) para los criterios de validación previos a una versión estable.
